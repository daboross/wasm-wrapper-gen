use std::fmt::Write;

use failure::Error;

use wasm_wrapper_gen_shared::{JsFnInfo, KnownArgumentType, TransformedRustIdent};

use self::indented_write::WriteExt;

pub fn generate_javascript<'a, 'b, I>(js_class_name: &str, iter: &'a I) -> Result<String, Error>
where
    &'a I: IntoIterator<Item = &'b JsFnInfo> + 'a,
{
    let mut output_buffer = String::new();
    {
        let buf = &mut output_buffer;

        write_class_definition_up_to_exports_grabbing(buf, js_class_name)?;

        {
            let buf = &mut buf.indented(12);
            for info in iter {
                write_func_unexport(buf, info)?;
            }
        }

        write_class_definition_post_exports_grabbing_up_to_methods(buf)?;

        {
            let buf = &mut buf.indented(4);
            for info in iter {
                write_method(buf, info)?;
            }
        }

        write_class_definition_finish(buf, js_class_name)?;
    }
    Ok(output_buffer)
}

fn write_class_definition_up_to_exports_grabbing<T>(
    buf: &mut T,
    js_class_name: &str,
) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        r#"class {} {{
    constructor(wasm_module) {{
        this._mod = new WebAssembly.Instance(wasm_module, {{
            // TODO: imports
        }});
        this._mem = new Uint8Array(this._mod.exports["memory"].buffer);

        this._alloc = this._mod.exports["__js_fn__builtin_alloc"];
        this._dealloc = this._mod.exports["__js_fn__builtin_dealloc"];

        this._funcs = {{
"#,
        js_class_name
    )?;
    Ok(())
}

fn write_func_unexport<T>(buf: &mut T, info: &JsFnInfo) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        "['{}']: this._mod.exports[\"{}\"],\n",
        info.rust_name,
        TransformedRustIdent::new(&info.rust_name)
    )?;

    Ok(())
}

fn write_class_definition_post_exports_grabbing_up_to_methods<T>(buf: &mut T) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        r#"        }};
    }}
"#
    )?;
    Ok(())
}

fn write_method<T>(buf: &mut T, info: &JsFnInfo) -> Result<(), Error>
where
    T: Write,
{
    write!(buf, "\n{}(", info.rust_name)?;
    let mut first_iteration = true;
    for (i, arg) in info.args_ty.iter().enumerate() {
        if !first_iteration {
            write!(buf, ", ")?;
        }
        match *arg {
            KnownArgumentType::U8SliceRef | KnownArgumentType::U8SliceMutRef => {
                write!(buf, "arg{}", i)?;
            }
        }
        first_iteration = false;
    }
    write!(buf, ") {{\n")?;

    {
        let buf = &mut buf.indented(4);
        // argument testing
        for (i, ty) in info.args_ty.iter().enumerate() {
            match *ty {
                KnownArgumentType::U8SliceRef | KnownArgumentType::U8SliceMutRef => {
                    write!(
                        buf,
                        r#"if (arg{0} == null || isNaN(arg{0}.length)) {{
    throw new Error();
}}
"#,
                        i
                    )?;
                }
            }
        }
        // allocation
        for (i, ty) in info.args_ty.iter().enumerate() {
            match *ty {
                KnownArgumentType::U8SliceRef | KnownArgumentType::U8SliceMutRef => {
                    // store length in temp variable
                    write!(buf, "let arg{0}_len = arg{0}.length;\n", i)?;
                    // allocate memory
                    write!(buf, "let arg{0}_ptr = this._alloc(arg{0}_len);\n", i)?;
                    // get a 'view' into the allocated memory, and copy the argument's data into it
                    write!(
                        buf,
                        "this._mem.subarray(arg{0}_ptr, arg{0}_ptr + arg{0}_len).set(arg{0});\n",
                        i
                    )?;
                }
            }
        }

        // actual function call
        write!(buf, "let result = this._funcs['{}'](", info.rust_name)?;
        let mut first_iteration = true;
        for (i, ty) in info.args_ty.iter().enumerate() {
            if !first_iteration {
                write!(buf, ", ")?;
            }

            match *ty {
                KnownArgumentType::U8SliceRef | KnownArgumentType::U8SliceMutRef => {
                    write!(buf, "arg{0}_ptr, arg{0}_len", i)?;
                }
            }
            first_iteration = false;
        }

        write!(buf, ");\n")?;

        // cleanup (deallocation)
        for (i, ty) in info.args_ty.iter().enumerate() {
            match *ty {
                KnownArgumentType::U8SliceRef => {
                    write!(buf, "this._dealloc(arg{0}_ptr, arg{0}_len);\n", i)?;
                }
                KnownArgumentType::U8SliceMutRef => {
                    // propagate modifications outwards.
                    write!(
                        buf,
                        r#"if (typeof arg{0}.set == 'function') {{
    arg{0}.set(this._mem.subarray(arg{0}_ptr, arg{0}_ptr + arg{0}_len));
}} else {{
    let arg{0}_view = this._mem.subarray(arg{0}_ptr, arg{0}_ptr + arg{0}_len);
    for (var i = 0; i < arg{0}_len; i++) {{
        arg{0}[i] = arg{0}_view[i];
    }}
}}
"#,
                        i
                    )?;
                    write!(buf, "this._dealloc(arg{0}_ptr, arg{0}_len);\n", i)?;
                }
            }
        }

        // TODO: handle return values (we can eventually do this by allocating
        // a Box<(*const ptr, usize)> of memory to store a (ptr, len) for the &[u8] returned)
        write!(buf, "return null;\n")?;
    }

    write!(buf, "}}\n")?;

    Ok(())
}


fn write_class_definition_finish<T>(buf: &mut T, js_class_name: &str) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        r#"}}

exports = module.exports = {}"#,
        js_class_name
    )?;
    Ok(())
}


mod indented_write {
    use std::fmt::{self, Write};

    pub struct IdentedWriter<T: Write> {
        inner: T,
        indent: u32,
        /// True if this writer hasn't written since the last new line.
        is_primed: bool,
    }
    impl<T: Write> IdentedWriter<T> {
        pub fn new(inner: T, indent: u32) -> Self {
            IdentedWriter {
                inner,
                indent,
                is_primed: true,
            }
        }
    }

    impl<T: Write> Write for IdentedWriter<T> {
        fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
            let mut iter = s.split("\n");

            // split always returns at least one character
            let first_part = iter.next().unwrap();
            if !first_part.is_empty() {
                if self.is_primed {
                    self.is_primed = false;
                    for _ in 0..self.indent {
                        self.inner.write_str(" ")?;
                    }
                }
                self.inner.write_str(first_part)?;
            }
            // if there were any newlines, there will be more than one item.
            //
            // write each newline and then write the indent before writing.
            for part in iter {
                self.inner.write_str("\n")?;
                if part.is_empty() {
                    self.is_primed = true;
                } else {
                    self.is_primed = false;
                    for _ in 0..self.indent {
                        self.inner.write_str(" ")?;
                    }
                    self.inner.write_str(part)?;
                }
            }

            Ok(())
        }
    }

    pub trait WriteExt: Write {
        fn indented(&mut self, indent: u32) -> IdentedWriter<&mut Self> {
            IdentedWriter::new(self, indent)
        }
    }
    impl<T: Write> WriteExt for T {}
}
