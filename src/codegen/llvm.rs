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

    // Stack of symbol tables for block scoping
    scopes: Vec<HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>>,
    // Forward-declared functions
    functions: HashMap<String, FunctionValue<'ctx>>,

    // Global String Type: { ptr, i64 }
    string_type: StructType<'ctx>,
    // Generic List Type struct: { ptr, i64, i64 } (buffer, capacity, length)
    list_struct_type: StructType<'ctx>,
    list_type: inkwell::types::PointerType<'ctx>,

    // Stack of active loop blocks (cond_bb, end_bb, target_depth) for skip/stop
    loop_blocks: Vec<(
        inkwell::basic_block::BasicBlock<'ctx>,
        inkwell::basic_block::BasicBlock<'ctx>,
        usize,
    )>,
}

impl<'ctx, 'a, 'b> LLVMCodegen<'ctx, 'a, 'b> {
    pub fn new(context: &'ctx Context, module_name: &str, env: &'a TypeEnv<'b>) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        // Define String struct { ptr, i64 }
        let ptr_type = context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = context.i64_type();
        let string_type = context.struct_type(&[ptr_type.into(), i64_type.into()], false);

        let list_struct_type =
            context.struct_type(&[ptr_type.into(), i64_type.into(), i64_type.into()], false);
        let list_type = list_struct_type.ptr_type(inkwell::AddressSpace::default());

        Self {
            context,
            module,
            builder,
            _env: env,
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            string_type,
            list_struct_type,
            list_type,
            loop_blocks: Vec::new(),
        }
    }

    fn resolve_variable(&self, name: &str) -> Option<&(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var);
            }
        }
        None
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope_and_free(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            for (_, (ptr, ty)) in scope {
                if ty == self.list_type.into() {
                    let list_ptr = self
                        .builder
                        .build_load(ty, ptr, "list_drop")
                        .unwrap()
                        .into_pointer_value();
                    let buffer_ptr_ptr = self
                        .builder
                        .build_struct_gep(self.list_struct_type, list_ptr, 0, "buf_gep")
                        .unwrap();
                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                    let buffer_ptr = self
                        .builder
                        .build_load(ptr_type, buffer_ptr_ptr, "buf")
                        .unwrap()
                        .into_pointer_value();
                    let free_func = self.module.get_function("free").unwrap();
                    self.builder
                        .build_call(free_func, &[buffer_ptr.into()], "")
                        .unwrap();
                    // Also free the list struct itself since it's heap allocated!
                    self.builder
                        .build_call(free_func, &[list_ptr.into()], "")
                        .unwrap();
                }
            }
        }
    }

    fn drop_scopes_for_return(&mut self) {
        for scope in self.scopes.iter().rev() {
            for (_, (ptr, ty)) in scope {
                if *ty == self.list_type.into() {
                    let list_ptr = self
                        .builder
                        .build_load(*ty, *ptr, "list_drop")
                        .unwrap()
                        .into_pointer_value();
                    let buffer_ptr_ptr = self
                        .builder
                        .build_struct_gep(self.list_struct_type, list_ptr, 0, "buf_gep")
                        .unwrap();
                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                    let buffer_ptr = self
                        .builder
                        .build_load(ptr_type, buffer_ptr_ptr, "buf")
                        .unwrap()
                        .into_pointer_value();
                    let free_func = self.module.get_function("free").unwrap();
                    self.builder
                        .build_call(free_func, &[buffer_ptr.into()], "")
                        .unwrap();
                    self.builder
                        .build_call(free_func, &[list_ptr.into()], "")
                        .unwrap();
                }
            }
        }
    }

    fn drop_scopes_for_loop(&mut self, target_depth: usize) {
        // Iterate backwards from the top scope down to `target_depth` (exclusive of target_depth).
        // For example, if target_depth is 2, we drop scopes 2, 3, etc. but keep scopes 0 and 1.
        for scope in self.scopes[target_depth..].iter().rev() {
            for (_, (ptr, ty)) in scope {
                if *ty == self.list_type.into() {
                    let list_ptr = self
                        .builder
                        .build_load(*ty, *ptr, "list_drop")
                        .unwrap()
                        .into_pointer_value();
                    let buffer_ptr_ptr = self
                        .builder
                        .build_struct_gep(self.list_struct_type, list_ptr, 0, "buf_gep")
                        .unwrap();
                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                    let buffer_ptr = self
                        .builder
                        .build_load(ptr_type, buffer_ptr_ptr, "buf")
                        .unwrap()
                        .into_pointer_value();
                    let free_func = self.module.get_function("free").unwrap();
                    self.builder
                        .build_call(free_func, &[buffer_ptr.into()], "")
                        .unwrap();
                    self.builder
                        .build_call(free_func, &[list_ptr.into()], "")
                        .unwrap();
                }
            }
        }
    }

    fn compile_ast_type(&self, ty: &ast::Type) -> Option<BasicTypeEnum<'ctx>> {
        match ty {
            ast::Type::Primitive(ast::PrimType::Int, _) => Some(self.context.i64_type().into()),
            ast::Type::Primitive(ast::PrimType::Float, _) => Some(self.context.f64_type().into()),
            ast::Type::Primitive(ast::PrimType::Bool, _) => Some(self.context.bool_type().into()),
            ast::Type::Primitive(ast::PrimType::String, _) => Some(self.string_type.into()),
            ast::Type::Generic { name, .. } if name == "List" => Some(self.list_type.into()),
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

        // Memory management (libc)
        let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
        self.module.add_function(
            "malloc",
            malloc_type,
            Some(inkwell::module::Linkage::External),
        );

        let realloc_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
        self.module.add_function(
            "realloc",
            realloc_type,
            Some(inkwell::module::Linkage::External),
        );

        let free_type = self.context.void_type().fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("free", free_type, Some(inkwell::module::Linkage::External));
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
        } else if f.name.starts_with("__builtin_math_") {
            f.name.strip_prefix("__builtin_math_").unwrap()
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

        self.scopes = vec![HashMap::new()];

        for (i, param) in f.params.iter().enumerate() {
            let arg_val = function.get_nth_param(i as u32).unwrap();
            let alloca = self
                .builder
                .build_alloca(arg_val.get_type(), &param.name)
                .unwrap();
            self.builder.build_store(alloca, arg_val).unwrap();
            self.scopes
                .last_mut()
                .unwrap()
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
            self.pop_scope_and_free(); // root scope
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
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(name.clone(), (alloca, llvm_ty));
            }
            ast::Stmt::ExprStmt(expr, _) => {
                self.compile_expr(expr)?;
            }
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
                if let Some((ptr, ty)) = self.resolve_variable(name) {
                    let val = self.builder.build_load(*ty, *ptr, name).unwrap();
                    Ok(Some(val))
                } else {
                    Err(format!("Unknown variable: {}", name))
                }
            }
            ast::Expr::Assign { target, value, .. } => {
                let val = self.compile_expr(value)?.unwrap();
                if let ast::Expr::Ident(name, _) = &**target {
                    if let Some((ptr, _ty)) = self.resolve_variable(name) {
                        self.builder.build_store(*ptr, val).unwrap();
                        Ok(Some(val))
                    } else {
                        Err(format!("Unknown variable: {}", name))
                    }
                } else if let ast::Expr::IndexAccess { object, index, .. } = &**target {
                    let list_ptr = self.compile_expr(object)?.unwrap().into_pointer_value();
                    let idx_val = self.compile_expr(index)?.unwrap().into_int_value();
                    let buf_ptr_gep = self
                        .builder
                        .build_struct_gep(self.list_struct_type, list_ptr, 0, "buf_gep")
                        .unwrap();
                    let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                    let buf_ptr = self
                        .builder
                        .build_load(ptr_type, buf_ptr_gep, "buf")
                        .unwrap()
                        .into_pointer_value();
                    let item_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(val.get_type(), buf_ptr, &[idx_val], "item_ptr")
                            .unwrap()
                    };
                    self.builder.build_store(item_ptr, val).unwrap();
                    Ok(Some(val))
                } else {
                    Err("Complex assignment not supported in codegen stub".into())
                }
            }
            ast::Expr::IndexAccess { object, index, .. } => {
                let list_ptr = self.compile_expr(object)?.unwrap().into_pointer_value();
                let idx_val = self.compile_expr(index)?.unwrap().into_int_value();
                let buf_ptr_gep = self
                    .builder
                    .build_struct_gep(self.list_struct_type, list_ptr, 0, "buf_gep")
                    .unwrap();
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let buf_ptr = self
                    .builder
                    .build_load(ptr_type, buf_ptr_gep, "buf")
                    .unwrap()
                    .into_pointer_value();
                let item_ptr = unsafe {
                    self.builder
                        .build_in_bounds_gep(
                            self.context.i64_type(),
                            buf_ptr,
                            &[idx_val],
                            "item_ptr",
                        )
                        .unwrap()
                };
                let item = self
                    .builder
                    .build_load(self.context.i64_type(), item_ptr, "item")
                    .unwrap();
                Ok(Some(item))
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
                    } else if name == "push" {
                        let list_ptr = self.compile_expr(&args[0])?.unwrap().into_pointer_value();
                        let item_val = self.compile_expr(&args[1])?.unwrap();
                        let len_ptr = self
                            .builder
                            .build_struct_gep(self.list_struct_type, list_ptr, 2, "len_ptr")
                            .unwrap();
                        let len = self
                            .builder
                            .build_load(self.context.i64_type(), len_ptr, "len")
                            .unwrap()
                            .into_int_value();
                        let buf_ptr_gep = self
                            .builder
                            .build_struct_gep(self.list_struct_type, list_ptr, 0, "buf_ptr_gep")
                            .unwrap();
                        let buf_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(inkwell::AddressSpace::default()),
                                buf_ptr_gep,
                                "buf",
                            )
                            .unwrap()
                            .into_pointer_value();
                        let item_ptr = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    item_val.get_type(),
                                    buf_ptr,
                                    &[len],
                                    "item_ptr",
                                )
                                .unwrap()
                        };
                        self.builder.build_store(item_ptr, item_val).unwrap();
                        let one = self.context.i64_type().const_int(1, false);
                        let new_len = self.builder.build_int_add(len, one, "new_len").unwrap();
                        self.builder.build_store(len_ptr, new_len).unwrap();
                        return Ok(None);
                    } else if name == "pop" {
                        let list_ptr = self.compile_expr(&args[0])?.unwrap().into_pointer_value();
                        let len_ptr = self
                            .builder
                            .build_struct_gep(self.list_struct_type, list_ptr, 2, "len_ptr")
                            .unwrap();
                        let len = self
                            .builder
                            .build_load(self.context.i64_type(), len_ptr, "len")
                            .unwrap()
                            .into_int_value();
                        let one = self.context.i64_type().const_int(1, false);
                        let new_len = self.builder.build_int_sub(len, one, "new_len").unwrap();
                        self.builder.build_store(len_ptr, new_len).unwrap();
                        let buf_ptr_gep = self
                            .builder
                            .build_struct_gep(self.list_struct_type, list_ptr, 0, "buf_ptr_gep")
                            .unwrap();
                        let buf_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(inkwell::AddressSpace::default()),
                                buf_ptr_gep,
                                "buf",
                            )
                            .unwrap()
                            .into_pointer_value();
                        let item_ptr = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    self.context.i64_type(),
                                    buf_ptr,
                                    &[new_len],
                                    "item_ptr",
                                )
                                .unwrap()
                        };
                        let item = self
                            .builder
                            .build_load(self.context.i64_type(), item_ptr, "item")
                            .unwrap();
                        return Ok(Some(item));
                    } else if name == "List" {
                        let malloc_func = self.module.get_function("malloc").unwrap();
                        let struct_size = self.context.i64_type().const_int(24, false);
                        let call = self
                            .builder
                            .build_call(malloc_func, &[struct_size.into()], "list_alloc")
                            .unwrap();
                        let list_ptr_val = match call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => v.into_pointer_value(),
                            _ => unreachable!(),
                        };
                        let init_cap = self.context.i64_type().const_int(1024, false);
                        let init_len = self.context.i64_type().const_int(0, false);
                        let item_size = self.context.i64_type().const_int(8, false);
                        let alloc_size = self
                            .builder
                            .build_int_mul(init_cap, item_size, "alloc_size")
                            .unwrap();
                        let buf_call = self
                            .builder
                            .build_call(malloc_func, &[alloc_size.into()], "buf_alloc")
                            .unwrap();
                        let buf_ptr_val = match buf_call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => v,
                            _ => unreachable!(),
                        };
                        let buf_gep = self
                            .builder
                            .build_struct_gep(self.list_struct_type, list_ptr_val, 0, "buf_gep")
                            .unwrap();
                        self.builder.build_store(buf_gep, buf_ptr_val).unwrap();
                        let cap_gep = self
                            .builder
                            .build_struct_gep(self.list_struct_type, list_ptr_val, 1, "cap_gep")
                            .unwrap();
                        self.builder.build_store(cap_gep, init_cap).unwrap();
                        let len_gep = self
                            .builder
                            .build_struct_gep(self.list_struct_type, list_ptr_val, 2, "len_gep")
                            .unwrap();
                        self.builder.build_store(len_gep, init_len).unwrap();
                        return Ok(Some(list_ptr_val.into()));
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
                    } else if let Some((ptr, _ty)) = self.resolve_variable(name) {
                        // Indirect call (Lambda)
                        let ptr_val = *ptr;
                        let mut compiled_args = Vec::new();
                        let mut arg_types = Vec::new();
                        for arg in args {
                            let arg_val = self.compile_expr(arg)?.unwrap();
                            compiled_args.push(arg_val.into());
                            arg_types.push(arg_val.get_type().into());
                        }

                        let func_ptr = self
                            .builder
                            .build_load(
                                self.context.ptr_type(inkwell::AddressSpace::default()),
                                ptr_val,
                                "load_func_ptr",
                            )
                            .unwrap()
                            .into_pointer_value();
                        let func_type = self.context.i64_type().fn_type(&arg_types, false);
                        let call = self
                            .builder
                            .build_indirect_call(func_type, func_ptr, &compiled_args, "calltmp")
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
            ast::Expr::If {
                condition,
                then_block,
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
                self.push_scope();
                let mut then_val = None;
                for s in &then_block.stmts {
                    self.compile_stmt(s)?;
                }
                if let Some(ref e) = then_block.expr {
                    then_val = self.compile_expr(e)?;
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.pop_scope_and_free();
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }
                let mut else_val = None;
                if else_block.is_some() {
                    self.builder.position_at_end(else_bb);
                    self.push_scope();
                    let e_block = else_block.as_ref().unwrap();
                    for s in &e_block.stmts {
                        self.compile_stmt(s)?;
                    }
                    if let Some(ref e) = e_block.expr {
                        else_val = self.compile_expr(e)?;
                    }
                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_none()
                    {
                        self.pop_scope_and_free();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                    }
                }
                self.builder.position_at_end(merge_bb);
                // Return value is simplified since phi node would be complex here, just return none for this basic port
                Ok(then_val.or(else_val))
            }
            ast::Expr::While {
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

                let target_depth = self.scopes.len();
                self.loop_blocks.push((cond_bb, end_bb, target_depth));
                self.push_scope();
                for s in &body.stmts {
                    self.compile_stmt(s)?;
                }
                if let Some(ref expr) = body.expr {
                    self.compile_expr(expr)?;
                }
                self.loop_blocks.pop();
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.pop_scope_and_free();
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }
                self.builder.position_at_end(end_bb);
                Ok(None)
            }
            ast::Expr::Block(block) => {
                self.push_scope();
                let mut val = None;
                for stmt in &block.stmts {
                    self.compile_stmt(stmt)?;
                }
                if let Some(ref expr) = block.expr {
                    val = self.compile_expr(expr)?;
                }
                self.pop_scope_and_free();
                Ok(val)
            }
            ast::Expr::Return { value, .. } => {
                if let Some(expr) = value {
                    let val = self.compile_expr(expr)?.unwrap();
                    self.drop_scopes_for_return();
                    self.builder.build_return(Some(&val)).unwrap();
                } else {
                    self.drop_scopes_for_return();
                    self.builder.build_return(None).unwrap();
                }
                Ok(None)
            }
            ast::Expr::Stop(_) => {
                if let Some(&(_, end_bb, target_depth)) = self.loop_blocks.last() {
                    self.drop_scopes_for_loop(target_depth);
                    self.builder.build_unconditional_branch(end_bb).unwrap();
                }
                Ok(None)
            }
            ast::Expr::Skip(_) => {
                if let Some(&(cond_bb, _, target_depth)) = self.loop_blocks.last() {
                    self.drop_scopes_for_loop(target_depth);
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }
                Ok(None)
            }
            ast::Expr::Match { subject, cases, .. } => {
                let subj_val = self.compile_expr(subject)?.unwrap();
                let parent = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let merge_bb = self.context.append_basic_block(parent, "matchcont");

                let mut current_bb = self.builder.get_insert_block().unwrap();

                for (i, case) in cases.iter().enumerate() {
                    let test_bb = self
                        .context
                        .append_basic_block(parent, &format!("match_test_{}", i));
                    let body_bb = self
                        .context
                        .append_basic_block(parent, &format!("match_body_{}", i));
                    let next_bb = self
                        .context
                        .append_basic_block(parent, &format!("match_next_{}", i));

                    self.builder.position_at_end(current_bb);
                    self.builder.build_unconditional_branch(test_bb).unwrap();

                    self.builder.position_at_end(test_bb);
                    let cond_val = match &case.pattern {
                        ast::Pattern::Literal(lit) => match lit {
                            ast::Literal::Int(v) => {
                                let cmp_val = self.context.i64_type().const_int(*v as u64, false);
                                self.builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        subj_val.into_int_value(),
                                        cmp_val,
                                        "cmp",
                                    )
                                    .unwrap()
                            }
                            ast::Literal::Bool(b) => {
                                let cmp_val = self.context.bool_type().const_int(*b as u64, false);
                                self.builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        subj_val.into_int_value(),
                                        cmp_val,
                                        "cmp",
                                    )
                                    .unwrap()
                            }
                            _ => self.context.bool_type().const_int(0, false),
                        },
                        ast::Pattern::Wildcard(_) | ast::Pattern::Binding(_, _) => {
                            self.context.bool_type().const_int(1, false)
                        }
                        _ => self.context.bool_type().const_int(0, false),
                    };

                    if let Some(guard) = &case.guard {
                        let guard_test_bb = self.context.append_basic_block(parent, "guard_test");
                        self.builder
                            .build_conditional_branch(cond_val, guard_test_bb, next_bb)
                            .unwrap();
                        self.builder.position_at_end(guard_test_bb);
                        let guard_val = self.compile_expr(guard)?.unwrap().into_int_value();
                        self.builder
                            .build_conditional_branch(guard_val, body_bb, next_bb)
                            .unwrap();
                    } else {
                        self.builder
                            .build_conditional_branch(cond_val, body_bb, next_bb)
                            .unwrap();
                    }

                    self.builder.position_at_end(body_bb);
                    self.push_scope();
                    if let ast::Pattern::Binding(name, _) = &case.pattern {
                        let alloc = self
                            .builder
                            .build_alloca(subj_val.get_type(), name)
                            .unwrap();
                        self.builder.build_store(alloc, subj_val).unwrap();
                        self.scopes
                            .last_mut()
                            .unwrap()
                            .insert(name.clone(), (alloc, subj_val.get_type()));
                    }

                    for stmt in &case.body.stmts {
                        self.compile_stmt(stmt)?;
                    }
                    if let Some(ref expr) = case.body.expr {
                        self.compile_expr(expr)?;
                    }

                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_none()
                    {
                        self.pop_scope_and_free();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                    }

                    current_bb = next_bb;
                }

                self.builder.position_at_end(current_bb);
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                Ok(None)
            }
            ast::Expr::InferBlock(block) | ast::Expr::TrainBlock(block) => {
                self.push_scope();
                let mut val = None;
                for stmt in &block.stmts {
                    self.compile_stmt(stmt)?;
                }
                if let Some(ref expr) = block.expr {
                    val = self.compile_expr(expr)?;
                }
                self.pop_scope_and_free();
                Ok(val)
            }
            ast::Expr::Select { .. } => Ok(None),
            ast::Expr::Lambda { params, body, .. } => {
                // Static non-capturing lambda compilation for Phase 3
                let mut arg_types = Vec::new();
                for param in params {
                    arg_types.push(
                        self.compile_ast_type(&param.ty)
                            .unwrap_or(self.context.i64_type().into())
                            .into(),
                    );
                }

                // Assume i64 return type for now (type inference during codegen will be fully solved in Phase 4)
                let func_type = self.context.i64_type().fn_type(&arg_types, false);
                let func_name = format!("__ferrite_lambda_{}", self.scopes.len());
                let func = self.module.add_function(&func_name, func_type, None);

                let current_bb = self.builder.get_insert_block().unwrap();
                let basic_block = self.context.append_basic_block(func, "entry");
                self.builder.position_at_end(basic_block);

                self.push_scope();
                for (i, arg) in func.get_param_iter().enumerate() {
                    let param_name = &params[i].name;
                    let alloca = self
                        .builder
                        .build_alloca(arg.get_type(), param_name)
                        .unwrap();
                    self.builder.build_store(alloca, arg).unwrap();
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(param_name.clone(), (alloca, arg.get_type()));
                }

                let val = self.compile_expr(body)?;
                self.pop_scope_and_free();

                if let Some(v) = val {
                    self.builder.build_return(Some(&v)).unwrap();
                } else {
                    self.builder.build_return(None).unwrap();
                }

                self.builder.position_at_end(current_bb);

                Ok(Some(func.as_global_value().as_pointer_value().into()))
            }
            _ => Ok(None),
        }
    }

    pub fn emit_to_file(&self, path: &Path) -> Result<(), String> {
        self.module.print_to_file(path).map_err(|e| e.to_string())
    }
}
