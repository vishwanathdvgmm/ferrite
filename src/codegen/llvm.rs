use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use inkwell::FloatPredicate;
use inkwell::IntPredicate;
use std::collections::HashMap;
use std::path::Path;

use crate::ast;
use crate::types::TypeEnv;

pub struct LLVMCodegen<'ctx, 'a, 'b> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    _env: &'a TypeEnv<'b>,

    // Symbol table mapping variables to their LLVM stack allocations and types
    variables: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
    // Forward-declared functions
    functions: HashMap<String, FunctionValue<'ctx>>,

    // Global String Type: { ptr, i64 }
    string_type: StructType<'ctx>,
}

impl<'ctx, 'a, 'b> LLVMCodegen<'ctx, 'a, 'b> {
    pub fn new(context: &'ctx Context, module_name: &str, env: &'a TypeEnv<'b>) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        // Define String struct { ptr, i64 }
        let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = context.i64_type();
        let string_type = context.struct_type(&[ptr_type.into(), i64_type.into()], false);

        Self {
            context,
            module,
            builder,
            _env: env,
            variables: HashMap::new(),
            functions: HashMap::new(),
            string_type,
        }
    }

    fn compile_ast_type(&self, ty: &ast::Type) -> Option<BasicTypeEnum<'ctx>> {
        match ty {
            ast::Type::Primitive(ast::PrimType::Int, _) => Some(self.context.i64_type().into()),
            ast::Type::Primitive(ast::PrimType::Float, _) => Some(self.context.f64_type().into()),
            ast::Type::Primitive(ast::PrimType::Bool, _) => Some(self.context.bool_type().into()),
            ast::Type::Primitive(ast::PrimType::String, _) => Some(self.string_type.into()),
            _ => None,
        }
    }

    pub fn compile_program(&mut self, program: &ast::Program) -> Result<(), String> {
        self.declare_runtime_functions();

        // Pass 1: Declare all functions and types
        for decl in &program.decls {
            if let ast::TopDecl::Func(f) = decl {
                self.declare_function(f)?;
            } else if let ast::TopDecl::TestFunc(f) = decl {
                self.declare_function(f)?;
            } else if let ast::TopDecl::ExternBlock(eb) = decl {
                for f in &eb.functions {
                    self.declare_extern_function(f)?;
                }
            }
        }

        // Pass 2: Definitions
        for decl in &program.decls {
            if let ast::TopDecl::Func(f) = decl {
                self.compile_function(f)?;
            } else if let ast::TopDecl::TestFunc(f) = decl {
                self.compile_function(f)?;
            }
        }

        if let Err(e) = self.module.verify() {
            return Err(format!(
                "LLVM Module verification failed: {}",
                e.to_string()
            ));
        }

        Ok(())
    }

    fn declare_runtime_functions(&mut self) {
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();

        let fn_type = ptr_type.fn_type(
            &[
                ptr_type.into(),
                i64_type.into(),
                ptr_type.into(),
                i64_type.into(),
            ],
            false,
        );
        self.module.add_function(
            "ferrite_string_concat",
            fn_type,
            Some(inkwell::module::Linkage::External),
        );

        let fn_type = ptr_type.fn_type(&[self.context.i64_type().into()], false);
        self.module.add_function(
            "ferrite_int_to_string",
            fn_type,
            Some(inkwell::module::Linkage::External),
        );

        let fn_type = ptr_type.fn_type(&[self.context.f64_type().into()], false);
        self.module.add_function(
            "ferrite_float_to_string",
            fn_type,
            Some(inkwell::module::Linkage::External),
        );

        let fn_type = self
            .context
            .void_type()
            .fn_type(&[ptr_type.into(), i64_type.into()], false);
        self.module.add_function(
            "ferrite_println",
            fn_type,
            Some(inkwell::module::Linkage::External),
        );

        let fn_type = self
            .context
            .void_type()
            .fn_type(&[ptr_type.into(), i64_type.into()], false);
        self.module.add_function(
            "ferrite_print",
            fn_type,
            Some(inkwell::module::Linkage::External),
        );
    }

    fn declare_function(&mut self, f: &ast::FuncDecl) -> Result<(), String> {
        let mut param_types = Vec::new();
        for p in &f.params {
            if let Some(ty) = self.compile_ast_type(&p.ty) {
                param_types.push(ty.into());
            } else {
                return Err(format!(
                    "Unsupported parameter type in function '{}'",
                    f.name
                ));
            }
        }

        let fn_type = if let Some(ret_ty) = &f.return_type {
            if let Some(ty) = self.compile_ast_type(ret_ty) {
                ty.fn_type(&param_types, false)
            } else {
                return Err(format!("Unsupported return type in function '{}'", f.name));
            }
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };

        let fn_name = if f.name == "main" {
            "ferrite_main".to_string()
        } else {
            f.name.clone()
        };
        let function = self.module.add_function(&fn_name, fn_type, None);
        self.functions.insert(f.name.clone(), function);
        Ok(())
    }

    fn declare_extern_function(&mut self, f: &ast::ExternFuncDecl) -> Result<(), String> {
        let mut param_types = Vec::new();
        for p in &f.params {
            if let Some(ty) = self.compile_ast_type(&p.ty) {
                param_types.push(ty.into());
            } else {
                param_types.push(self.context.i64_type().into());
            }
        }
        let fn_type = if let Some(ret_ty) = &f.return_type {
            if let Some(ty) = self.compile_ast_type(ret_ty) {
                ty.fn_type(&param_types, false)
            } else {
                self.context.void_type().fn_type(&param_types, false)
            }
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };

        let fn_name = if f.name == "main" {
            "ferrite_main"
        } else {
            &f.name
        };

        let function =
            self.module
                .add_function(fn_name, fn_type, Some(inkwell::module::Linkage::External));
        self.functions.insert(f.name.clone(), function);
        Ok(())
    }

    fn compile_function(&mut self, f: &ast::FuncDecl) -> Result<(), String> {
        let function = *self.functions.get(&f.name).unwrap();

        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        self.variables.clear();

        for (i, param) in f.params.iter().enumerate() {
            let arg_val = function.get_nth_param(i as u32).unwrap();
            let alloca = self
                .builder
                .build_alloca(arg_val.get_type(), &param.name)
                .unwrap();
            self.builder.build_store(alloca, arg_val).unwrap();
            self.variables
                .insert(param.name.clone(), (alloca, arg_val.get_type()));
        }

        for stmt in &f.body.stmts {
            self.compile_stmt(stmt)?;
        }

        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            if f.return_type.is_none() {
                self.builder.build_return(None).unwrap();
            } else {
                self.builder.build_unreachable().unwrap();
            }
        }

        if !function.verify(true) {
            return Err(format!("Function verification failed for '{}'", f.name));
        }

        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &ast::Stmt) -> Result<(), String> {
        match stmt {
            ast::Stmt::Keep {
                name, ty, value, ..
            }
            | ast::Stmt::Param {
                name, ty, value, ..
            } => {
                let init_val = self.compile_expr(value)?.unwrap();
                let llvm_ty = self.compile_ast_type(ty).unwrap_or(init_val.get_type());
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.builder.build_store(alloca, init_val).unwrap();
                self.variables.insert(name.clone(), (alloca, llvm_ty));
            }
            ast::Stmt::ExprStmt(expr) => {
                self.compile_expr(expr)?;
            }
            ast::Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    let val = self.compile_expr(expr)?.unwrap();
                    self.builder.build_return(Some(&val)).unwrap();
                } else {
                    self.builder.build_return(None).unwrap();
                }
            }
            ast::Stmt::If {
                condition,
                then_block,
                elif_branches: _,
                else_block,
                ..
            } => {
                let cond_val = self.compile_expr(condition)?.unwrap().into_int_value();
                let parent = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                let then_bb = self.context.append_basic_block(parent, "then");
                let merge_bb = self.context.append_basic_block(parent, "ifcont");
                let mut else_bb = merge_bb;

                if else_block.is_some() {
                    else_bb = self.context.append_basic_block(parent, "else");
                }

                self.builder
                    .build_conditional_branch(cond_val, then_bb, else_bb)
                    .unwrap();

                self.builder.position_at_end(then_bb);
                for s in &then_block.stmts {
                    self.compile_stmt(s)?;
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                if else_block.is_some() {
                    self.builder.position_at_end(else_bb);
                    for s in else_block.as_ref().unwrap().stmts.iter() {
                        self.compile_stmt(s)?;
                    }
                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_none()
                    {
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                    }
                }

                self.builder.position_at_end(merge_bb);
            }
            ast::Stmt::While {
                condition, body, ..
            } => {
                let parent = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let cond_bb = self.context.append_basic_block(parent, "whilecond");
                let loop_bb = self.context.append_basic_block(parent, "whileloop");
                let end_bb = self.context.append_basic_block(parent, "whileend");

                self.builder.build_unconditional_branch(cond_bb).unwrap();
                self.builder.position_at_end(cond_bb);

                let cond_val = self.compile_expr(condition)?.unwrap().into_int_value();
                self.builder
                    .build_conditional_branch(cond_val, loop_bb, end_bb)
                    .unwrap();

                self.builder.position_at_end(loop_bb);
                for s in &body.stmts {
                    self.compile_stmt(s)?;
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }

                self.builder.position_at_end(end_bb);
            }
            _ => {}
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &ast::Expr) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match expr {
            ast::Expr::Lit(lit, _) => match lit {
                ast::Literal::Int(n) => {
                    let i64_type = self.context.i64_type();
                    Ok(Some(i64_type.const_int(*n as u64, true).into()))
                }
                ast::Literal::Float(f) => {
                    let f64_type = self.context.f64_type();
                    Ok(Some(f64_type.const_float(*f).into()))
                }
                ast::Literal::Bool(b) => {
                    let bool_type = self.context.bool_type();
                    Ok(Some(bool_type.const_int(*b as u64, false).into()))
                }
                ast::Literal::String(s) => {
                    let len = s.len() as u64;
                    let i8_type = self.context.i8_type();
                    let mut bytes = s.as_bytes().to_vec();
                    bytes.push(0);
                    let array_type = i8_type.array_type(bytes.len() as u32);
                    let chars = bytes
                        .iter()
                        .map(|&b| i8_type.const_int(b as u64, false))
                        .collect::<Vec<_>>();
                    let array_val = i8_type.const_array(&chars);

                    let global = self.module.add_global(array_type, None, ".str");
                    global.set_initializer(&array_val);
                    global.set_linkage(inkwell::module::Linkage::Private);
                    global.set_constant(true);

                    let zero = self.context.i32_type().const_int(0, false);
                    let gep = unsafe {
                        self.builder.build_in_bounds_gep(
                            array_type,
                            global.as_pointer_value(),
                            &[zero, zero],
                            "strptr",
                        )
                    }
                    .unwrap();

                    let mut str_struct = self.string_type.get_undef();
                    str_struct = self
                        .builder
                        .build_insert_value(str_struct, gep, 0, "insert_ptr")
                        .unwrap()
                        .into_struct_value();
                    str_struct = self
                        .builder
                        .build_insert_value(
                            str_struct,
                            self.context.i64_type().const_int(len, false),
                            1,
                            "insert_len",
                        )
                        .unwrap()
                        .into_struct_value();

                    Ok(Some(str_struct.into()))
                }
            },
            ast::Expr::Ident(name, _) => {
                if let Some((ptr, ty)) = self.variables.get(name) {
                    let val = self.builder.build_load(*ty, *ptr, name).unwrap();
                    Ok(Some(val))
                } else {
                    Err(format!("Unknown variable: {}", name))
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                let val = self.compile_expr(value)?.unwrap();
                if let ast::Expr::Ident(name, _) = &**target {
                    if let Some((ptr, _ty)) = self.variables.get(name) {
                        self.builder.build_store(*ptr, val).unwrap();
                        Ok(Some(val))
                    } else {
                        Err(format!("Unknown variable: {}", name))
                    }
                } else {
                    Err("Complex assignment not supported in codegen stub".into())
                }
            }
            ast::Expr::Call { callee, args, .. } => {
                if let ast::Expr::Ident(name, _) = &**callee {
                    // Handle Builtins
                    if name == "println" || name == "print" {
                        let arg_val = self.compile_expr(&args[0])?.unwrap();
                        let func_name = if name == "println" {
                            "ferrite_println"
                        } else {
                            "ferrite_print"
                        };
                        let func = self.module.get_function(func_name).unwrap();

                        let str_struct = arg_val.into_struct_value();
                        let ptr_val = self
                            .builder
                            .build_extract_value(str_struct, 0, "ptr")
                            .unwrap();
                        let len_val = self
                            .builder
                            .build_extract_value(str_struct, 1, "len")
                            .unwrap();

                        self.builder
                            .build_call(func, &[ptr_val.into(), len_val.into()], "")
                            .unwrap();
                        return Ok(None);
                    } else if name == "str" {
                        let arg_val = self.compile_expr(&args[0])?.unwrap();
                        if arg_val.is_int_value() {
                            let func = self.module.get_function("ferrite_int_to_string").unwrap();
                            let call = self
                                .builder
                                .build_call(func, &[arg_val.into()], "str_val_ptr")
                                .unwrap();
                            let ptr_val = match call.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                                _ => unreachable!(),
                            };
                            let loaded_struct = self
                                .builder
                                .build_load(self.string_type, ptr_val, "loaded_str")
                                .unwrap();
                            return Ok(Some(loaded_struct));
                        } else if arg_val.is_float_value() {
                            let func = self.module.get_function("ferrite_float_to_string").unwrap();
                            let call = self
                                .builder
                                .build_call(func, &[arg_val.into()], "str_val_ptr")
                                .unwrap();
                            let ptr_val = match call.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                                _ => unreachable!(),
                            };
                            let loaded_struct = self
                                .builder
                                .build_load(self.string_type, ptr_val, "loaded_str")
                                .unwrap();
                            return Ok(Some(loaded_struct));
                        }
                    } else if name == "len" {
                        let arg_val = self.compile_expr(&args[0])?.unwrap();
                        if arg_val.is_struct_value() {
                            let len_val = self
                                .builder
                                .build_extract_value(arg_val.into_struct_value(), 1, "len")
                                .unwrap();
                            return Ok(Some(len_val.into()));
                        } else {
                            return Err("len() called on non-string type".into());
                        }
                    }

                    if let Some(func) = self.functions.get(name).copied() {
                        let mut compiled_args = Vec::new();
                        for arg in args {
                            compiled_args.push(self.compile_expr(arg)?.unwrap().into());
                        }
                        let call = self
                            .builder
                            .build_call(func, &compiled_args, "calltmp")
                            .unwrap();
                        match call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => Ok(Some(v)),
                            _ => Ok(None),
                        }
                    } else {
                        Err(format!("Unknown function: {}", name))
                    }
                } else {
                    Err("Indirect calls not supported".into())
                }
            }
            ast::Expr::BinOp {
                left, op, right, ..
            } => {
                let lhs = self.compile_expr(left)?.unwrap();
                let rhs = self.compile_expr(right)?.unwrap();

                if lhs.is_struct_value() && rhs.is_struct_value() {
                    // String concat
                    if *op == ast::BinOp::Add {
                        let func = self.module.get_function("ferrite_string_concat").unwrap();
                        let a_struct = lhs.into_struct_value();
                        let b_struct = rhs.into_struct_value();

                        let a_ptr = self
                            .builder
                            .build_extract_value(a_struct, 0, "a_ptr")
                            .unwrap();
                        let a_len = self
                            .builder
                            .build_extract_value(a_struct, 1, "a_len")
                            .unwrap();
                        let b_ptr = self
                            .builder
                            .build_extract_value(b_struct, 0, "b_ptr")
                            .unwrap();
                        let b_len = self
                            .builder
                            .build_extract_value(b_struct, 1, "b_len")
                            .unwrap();

                        let call = self
                            .builder
                            .build_call(
                                func,
                                &[a_ptr.into(), a_len.into(), b_ptr.into(), b_len.into()],
                                "concat_ptr",
                            )
                            .unwrap();

                        let ptr_val = match call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                            _ => unreachable!(),
                        };
                        let loaded_struct = self
                            .builder
                            .build_load(self.string_type, ptr_val, "loaded_concat_str")
                            .unwrap();
                        return Ok(Some(loaded_struct));
                    }
                }

                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => match op {
                        ast::BinOp::Add => Ok(Some(
                            self.builder.build_int_add(l, r, "addtmp").unwrap().into(),
                        )),
                        ast::BinOp::Sub => Ok(Some(
                            self.builder.build_int_sub(l, r, "subtmp").unwrap().into(),
                        )),
                        ast::BinOp::Mul => Ok(Some(
                            self.builder.build_int_mul(l, r, "multmp").unwrap().into(),
                        )),
                        ast::BinOp::Div => Ok(Some(
                            self.builder
                                .build_int_signed_div(l, r, "divtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Mod => Ok(Some(
                            self.builder
                                .build_int_signed_rem(l, r, "modtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::And => {
                            Ok(Some(self.builder.build_and(l, r, "andtmp").unwrap().into()))
                        }
                        ast::BinOp::Eq => Ok(Some(
                            self.builder
                                .build_int_compare(IntPredicate::EQ, l, r, "eqtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::NotEq => Ok(Some(
                            self.builder
                                .build_int_compare(IntPredicate::NE, l, r, "neqtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Lt => Ok(Some(
                            self.builder
                                .build_int_compare(IntPredicate::SLT, l, r, "lttmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Gt => Ok(Some(
                            self.builder
                                .build_int_compare(IntPredicate::SGT, l, r, "gttmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::LtEq => Ok(Some(
                            self.builder
                                .build_int_compare(IntPredicate::SLE, l, r, "letmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::GtEq => Ok(Some(
                            self.builder
                                .build_int_compare(IntPredicate::SGE, l, r, "getmp")
                                .unwrap()
                                .into(),
                        )),
                        _ => Ok(None),
                    },
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => match op {
                        ast::BinOp::Add => Ok(Some(
                            self.builder
                                .build_float_add(l, r, "faddtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Sub => Ok(Some(
                            self.builder
                                .build_float_sub(l, r, "fsubtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Mul => Ok(Some(
                            self.builder
                                .build_float_mul(l, r, "fmultmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Div => Ok(Some(
                            self.builder
                                .build_float_div(l, r, "fdivtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Mod => Ok(Some(
                            self.builder
                                .build_float_rem(l, r, "fmodtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Eq => Ok(Some(
                            self.builder
                                .build_float_compare(FloatPredicate::OEQ, l, r, "eqtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::NotEq => Ok(Some(
                            self.builder
                                .build_float_compare(FloatPredicate::ONE, l, r, "neqtmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Lt => Ok(Some(
                            self.builder
                                .build_float_compare(FloatPredicate::OLT, l, r, "lttmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::Gt => Ok(Some(
                            self.builder
                                .build_float_compare(FloatPredicate::OGT, l, r, "gttmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::LtEq => Ok(Some(
                            self.builder
                                .build_float_compare(FloatPredicate::OLE, l, r, "letmp")
                                .unwrap()
                                .into(),
                        )),
                        ast::BinOp::GtEq => Ok(Some(
                            self.builder
                                .build_float_compare(FloatPredicate::OGE, l, r, "getmp")
                                .unwrap()
                                .into(),
                        )),
                        _ => Ok(None),
                    },
                    _ => Err("Invalid binary operation types".into()),
                }
            }
            ast::Expr::UnsafeBlock(block, _) => {
                let last_val = None;
                for stmt in &block.stmts {
                    self.compile_stmt(stmt)?;
                }
                Ok(last_val)
            }
            ast::Expr::UnaryOp { op, operand, .. } => {
                let inner = self.compile_expr(operand)?.unwrap();
                match op {
                    ast::UnaryOp::Neg => {
                        if inner.is_int_value() {
                            Ok(Some(
                                self.builder
                                    .build_int_neg(inner.into_int_value(), "negtmp")
                                    .unwrap()
                                    .into(),
                            ))
                        } else if inner.is_float_value() {
                            Ok(Some(
                                self.builder
                                    .build_float_neg(inner.into_float_value(), "fnegtmp")
                                    .unwrap()
                                    .into(),
                            ))
                        } else {
                            Err("Invalid type for negation".into())
                        }
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    pub fn emit_to_file(&self, path: &Path) -> Result<(), String> {
        self.module.print_to_file(path).map_err(|e| e.to_string())
    }
}
