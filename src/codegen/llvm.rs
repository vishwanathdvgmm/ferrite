use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use std::collections::HashMap;
use std::path::Path;

use crate::ast;
use crate::types::TypeEnv;

pub struct LLVMCodegen<'ctx, 'a> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    env: &'a TypeEnv<'a>,

    // Symbol table mapping variables to their LLVM stack allocations
    variables: HashMap<String, PointerValue<'ctx>>,
    // Forward-declared functions
    functions: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx, 'a> LLVMCodegen<'ctx, 'a> {
    pub fn new(context: &'ctx Context, module_name: &str, env: &'a TypeEnv<'a>) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
            env,
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    /// Compile a generic AST program. Phase 1 provides structural stubs.
    pub fn compile_program(&mut self, program: &ast::Program) -> Result<(), String> {
        // Pass 1: Declare all functions and types
        // (LLVM requires forward declarations)
        for decl in &program.decls {
            if let ast::TopDecl::Func(f) = decl {
                self.declare_function(f)?;
            }
        }

        // Pass 2: Compile function bodies
        for decl in &program.decls {
            if let ast::TopDecl::Func(f) = decl {
                self.compile_function(f)?;
            }
        }

        // Verify the entire module
        if let Err(e) = self.module.verify() {
            return Err(format!(
                "LLVM Module verification failed: {}",
                e.to_string()
            ));
        }

        Ok(())
    }

    fn declare_function(&mut self, f: &ast::FuncDecl) -> Result<(), String> {
        // Default to `void` for phase 1 stub if mapping fails
        let void_type = self.context.void_type();
        let fn_type = void_type.fn_type(&[], false); // no params for now
        let function = self.module.add_function(&f.name, fn_type, None);
        self.functions.insert(f.name.clone(), function);
        Ok(())
    }

    fn compile_function(&mut self, f: &ast::FuncDecl) -> Result<(), String> {
        let function = self.functions.get(&f.name).unwrap();

        let basic_block = self.context.append_basic_block(*function, "entry");
        self.builder.position_at_end(basic_block);

        // Compile instructions...
        for stmt in &f.body.stmts {
            self.compile_stmt(stmt)?;
        }

        // Ensure returning at the end
        self.builder.build_return(None);

        // Function-level verification
        if !function.verify(true) {
            return Err(format!("Function verification failed for '{}'", f.name));
        }

        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &ast::Stmt) -> Result<(), String> {
        match stmt {
            ast::Stmt::Keep { name, .. } => {
                // alloc, store
            }
            ast::Stmt::ExprStmt(expr) => {
                self.compile_expr(expr)?;
            }
            ast::Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    let val = self.compile_expr(expr)?;
                    // self.builder.build_return(Some(&val));
                } else {
                    self.builder.build_return(None);
                }
            }
            // Other statements follow similar pattern mapping to LLVM
            _ => {}
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &ast::Expr) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        match expr {
            ast::Expr::Lit(lit, _) => {
                match lit {
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
                    ast::Literal::String(_) => {
                        // Ignoring strings in Phase 1 stub
                        Ok(None)
                    }
                }
            }
            ast::Expr::BinOp {
                left, op, right, ..
            } => {
                let lhs = self.compile_expr(left)?.unwrap();
                let rhs = self.compile_expr(right)?.unwrap();

                // Switch dynamically based on Int vs Float values
                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        match op {
                            ast::BinOp::Add => {
                                Ok(Some(self.builder.build_int_add(l, r, "addtmp").into()))
                            }
                            ast::BinOp::Sub => {
                                Ok(Some(self.builder.build_int_sub(l, r, "subtmp").into()))
                            }
                            ast::BinOp::Mul => {
                                Ok(Some(self.builder.build_int_mul(l, r, "multmp").into()))
                            }
                            ast::BinOp::Div => Ok(Some(
                                self.builder.build_int_signed_div(l, r, "divtmp").into(),
                            )),
                            _ => Ok(None), // ...
                        }
                    }
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                        match op {
                            ast::BinOp::Add => {
                                Ok(Some(self.builder.build_float_add(l, r, "faddtmp").into()))
                            }
                            ast::BinOp::Sub => {
                                Ok(Some(self.builder.build_float_sub(l, r, "fsubtmp").into()))
                            }
                            ast::BinOp::Mul => {
                                Ok(Some(self.builder.build_float_mul(l, r, "fmultmp").into()))
                            }
                            ast::BinOp::Div => {
                                Ok(Some(self.builder.build_float_div(l, r, "fdivtmp").into()))
                            }
                            _ => Ok(None), // ...
                        }
                    }
                    _ => Err("Invalid binary operation types".into()),
                }
            }
            _ => Ok(None),
        }
    }

    /// Emits the module to a native object file or bitcode
    pub fn emit_to_file(&self, path: &Path) -> Result<(), String> {
        self.module.print_to_file(path).map_err(|e| e.to_string())
    }
}
