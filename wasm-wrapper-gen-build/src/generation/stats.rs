use wasm_wrapper_gen_shared::{JsFnInfo, SupportedArgumentType, SupportedRetType};

pub(crate) struct FuncStats<'a> {
    pub inner: &'a JsFnInfo,
    pub uses_memory_access: bool,
    pub uses_post_function_memory_access: bool,
}

impl<'a> FuncStats<'a> {
    pub fn new(stats: &'a JsFnInfo) -> Self {
        let mut any_alloc = false;
        let mut post_func_mem_access = false;
        for arg in &stats.args_ty {
            match *arg {
                SupportedArgumentType::Integer(_) => {}
                SupportedArgumentType::IntegerSliceRef(_)
                | SupportedArgumentType::IntegerVec(_) => {
                    any_alloc = true;
                }
                SupportedArgumentType::IntegerSliceMutRef(_) => {
                    any_alloc = true;
                    post_func_mem_access = true;
                }
            }
        }
        match stats.ret_ty {
            SupportedRetType::Unit | SupportedRetType::Integer(_) => {}
            SupportedRetType::IntegerVec(_) => {
                any_alloc = true;
                post_func_mem_access = true;
            }
        }

        FuncStats {
            inner: stats,
            uses_memory_access: any_alloc,
            uses_post_function_memory_access: post_func_mem_access,
        }
    }
}

impl<'a> ::std::ops::Deref for FuncStats<'a> {
    type Target = JsFnInfo;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}
