mod stats;

use self::stats::FuncStats;
use std::fmt::{self, Display, Write};

use failure::Error;

use wasm_wrapper_gen_shared::{JsFnInfo, SupportedArgumentType, SupportedCopyTy, SupportedRetType,
                              TransformedRustIdent};

use style::{AccessStyle, Config};

use self::indented_write::WriteExt;

pub fn generate_javascript<'a, 'b, I>(config: &Config, iter: &'a I) -> Result<String, Error>
where
    &'a I: IntoIterator<Item = &'b JsFnInfo> + 'a,
{
    let mut any_alloc = false;
    let func_stats = iter.into_iter()
        .map(|info| {
            let stat = FuncStats::new(info);
            if stat.uses_memory_access {
                any_alloc = true;
            }
            stat
        })
        .collect::<Vec<_>>();

    let mut output_buffer = String::new();
    {
        let buf = &mut output_buffer;

        write_class_definition_up_to_exports_grabbing(config, buf, any_alloc)?;

        {
            let buf = &mut buf.indented(config.indent * 3);
            for stat in &func_stats {
                write_func_unexport(buf, stat.inner, stat)?;
            }
        }

        write_class_definition_post_exports_grabbing_up_to_methods(config, buf, any_alloc)?;

        {
            let buf = &mut buf.indented(config.indent);
            for stat in &func_stats {
                write_method(config, buf, stat.inner, stat)?;
            }
        }

        write_class_definition_finish(config, buf)?;
    }
    Ok(output_buffer)
}

fn write_class_definition_up_to_exports_grabbing<T>(
    config: &Config,
    buf: &mut T,
    any_alloc: bool,
) -> Result<(), Error>
where
    T: Write,
{
    write!(buf, "class {} {{\n", config.class_name)?;
    {
        let mut buf = buf.indented(config.indent);
        write!(buf, "constructor (wasm_module) {{\n")?;
        {
            let mut buf = buf.indented(config.indent);
            write!(
                buf,
                "this._mod = new WebAssembly.Instance(wasm_module, {{}});\n"
            )?;
            if any_alloc {
                match config.access_style {
                    AccessStyle::TypedArrays => {
                        write!(buf, "this._mem = this._mod.exports[\"memory\"];\n")?;
                    }
                    AccessStyle::DataView => {
                        write!(
                            buf,
                            r#"this._raw_mem = this._mod.exports["memory"];
this._mem = new DataView(this._raw_mem.buffer);
"#
                        )?;
                    }
                }
                write!(
                    buf,
                    r#"
this._alloc = this._mod.exports["__js_fn__builtin_alloc"];
this._dealloc = this._mod.exports["__js_fn__builtin_dealloc"];
"#
                )?;
            }
            write!(
                buf,
                r#"
this._funcs = {{
"#
            )?;
        }
    }
    Ok(())
}

fn write_func_unexport<T>(buf: &mut T, info: &JsFnInfo, _stats: &FuncStats) -> Result<(), Error>
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

fn write_class_definition_post_exports_grabbing_up_to_methods<T>(
    config: &Config,
    buf: &mut T,
    any_alloc: bool,
) -> Result<(), Error>
where
    T: Write,
{
    write!(buf.indented(config.indent * 2), "}};\n")?;
    write!(buf.indented(config.indent), "}}\n")?;
    match config.access_style {
        AccessStyle::TypedArrays => {}
        AccessStyle::DataView => {
            if any_alloc {
                let buf = &mut buf.indented(config.indent);
                write!(buf, "\n_check_mem_realloc() {{\n")?;
                {
                    let buf = &mut buf.indented(config.indent);
                    write!(
                        buf,
                        "if (this._mem.byteLength != this._raw_mem.byteLength) {{\n"
                    )?;
                    write!(
                        buf.indented(config.indent),
                        "this._mem = new DataView(this._raw_mem.buffer);\n"
                    )?;
                    write!(buf, "}}\n")?;
                }
                write!(buf, "}}\n")?;
            }
        }
    }
    Ok(())
}

fn validate_argument<T, U, V>(
    config: &Config,
    buf: &mut T,
    arg_name: U,
    ty: SupportedArgumentType,
    failure: V,
) -> fmt::Result
where
    T: Write,
    U: Display,
    V: Display,
{
    match ty {
        SupportedArgumentType::IntegerSliceRef(_)
        | SupportedArgumentType::IntegerSliceMutRef(_)
        | SupportedArgumentType::IntegerVec(_) => {
            write!(buf, "if ({0} == null || isNaN({0}).length) {{\n", arg_name)?;
            write!(buf.indented(config.indent), "{}\n", failure)?;
            write!(buf, "}}\n")?;
        }
        SupportedArgumentType::Integer(_) => {
            write!(buf, "if (isNaN({0})) {{\n", arg_name)?;
            write!(buf.indented(config.indent), "{}\n", failure)?;
            write!(buf, "}}\n")?;
        }
    }

    Ok(())
}

fn prepare_argument_allocation<T, U>(
    config: &Config,
    buf: &mut T,
    arg_name: U,
    ty: SupportedArgumentType,
) -> fmt::Result
where
    T: Write,
    U: Display,
{
    match ty {
        SupportedArgumentType::IntegerSliceRef(int_ty)
        | SupportedArgumentType::IntegerSliceMutRef(int_ty)
        | SupportedArgumentType::IntegerVec(int_ty) => match config.access_style {
            AccessStyle::TypedArrays => {
                write!(
                    buf,
                    r#"let {0}_len = {0}.length;
let {0}_byte_len = {0}_len * {1};
let {0}_ptr = this._alloc({0}_byte_len);
let {0}_view = new {2}(this._mem.buffer, {0}_ptr, {0}_byte_len);
{0}_view.set({0});
"#,
                    arg_name,
                    int_ty.size_in_bytes(),
                    javascript_typed_array_for_int(int_ty)
                )?;
            }
            AccessStyle::DataView => {
                write!(
                    buf,
                    r#"let {0}_len = {0}.length;
let {0}_byte_len = {0}_len * {1};
let {0}_ptr = this._alloc({0}_byte_len);
this._check_mem_realloc();
for (var {0}_i = 0; {0}_i < {0}_len; {0}_i++) {{
"#,
                    arg_name,
                    int_ty.size_in_bytes()
                )?;
                js_set_ith_ty_at(
                    buf.indented(config.indent),
                    "this._mem",
                    int_ty,
                    format_args!("{0}_ptr", arg_name),
                    format_args!("{0}_i", arg_name),
                    format_args!("{0}[{0}_i]", arg_name),
                )?;
                write!(buf, "}}\n")?;
            }
        },
        SupportedArgumentType::Integer(_) => {} // no allocation needed for integers.
    }

    Ok(())
}

fn deallocate_argument_allocation<T, U>(
    _config: &Config,
    buf: &mut T,
    arg_name: U,
    ty: SupportedArgumentType,
) -> fmt::Result
where
    T: Write,
    U: Display,
{
    // deallocate
    match ty {
        SupportedArgumentType::Integer(_) | SupportedArgumentType::IntegerVec(_) => {}
        SupportedArgumentType::IntegerSliceRef(_)
        | SupportedArgumentType::IntegerSliceMutRef(_) => {
            write!(buf, "this._dealloc({0}_ptr, {0}_byte_len);\n", arg_name)?;
        }
    }

    Ok(())
}

fn reconstruct_typed_array_if_memory_changed<T, U, V, W>(
    config: &Config,
    buf: &mut T,
    view_name: U,
    ptr_name: V,
    length_name: W,
    ty: SupportedCopyTy,
) -> fmt::Result
where
    T: Write,
    U: Display,
    V: Display,
    W: Display,
{
    write!(
        buf,
        "if ({0}.buffer.byteLength != this._mem.buffer.byteLength) {{\n",
        view_name
    )?;
    write!(
        buf.indented(config.indent),
        "{0} = new {1}(this._mem.buffer, {2}, {3});\n",
        view_name,
        javascript_typed_array_for_int(ty),
        ptr_name,
        length_name
    )?;
    write!(buf, "}}\n")?;

    Ok(())
}

fn propogate_argument_changes_outwards<T, U>(
    config: &Config,
    buf: &mut T,
    arg_name: U,
    ty: SupportedArgumentType,
) -> fmt::Result
where
    T: Write,
    U: Display,
{
    // copy changes back for mutable references
    match ty {
        SupportedArgumentType::IntegerSliceMutRef(int_ty) => {
            // propagate modifications outwards.
            match config.access_style {
                AccessStyle::TypedArrays => {
                    reconstruct_typed_array_if_memory_changed(
                        config,
                        buf,
                        format_args!("{}_view", arg_name),
                        format_args!("{}_ptr", arg_name),
                        format_args!("{}_byte_len", arg_name),
                        int_ty,
                    )?;
                    write!(buf, "if (typeof {0}.set == 'function') {{", arg_name)?;
                    write!(
                        buf.indented(config.indent),
                        "{0}.set({0}_view);\n",
                        arg_name
                    )?;
                    write!(buf, "}} else {{")?;
                    {
                        let mut buf = buf.indented(config.indent);

                        write!(
                            buf,
                            "for (var {0}_i = 0; {0}_i < {0}_len; {0}_i++) {{\n",
                            arg_name
                        )?;
                        {
                            let mut buf = buf.indented(config.indent);
                            write!(buf, "{0}[{0}_i] = ", arg_name)?;
                            if int_ty == SupportedCopyTy::Bool {
                                write!(buf, "Boolean({0}_view[{0}_i])", arg_name)?;
                            } else {
                                write!(buf, "{0}_view[{0}_i]", arg_name)?;
                            }
                            write!(buf, ";\n")?;
                        }
                        write!(buf, "}}\n")?;
                    }
                    write!(buf, "}}\n")?;
                }
                AccessStyle::DataView => {
                    write!(
                        buf,
                        "for (var {0}_i = 0; {0}_i < {0}_len; {0}_i++) {{\n",
                        arg_name
                    )?;
                    {
                        let mut buf = buf.indented(config.indent);
                        write!(buf, "{0}[{0}_i] = ", arg_name)?;
                        js_get_ith_ty_at(
                            &mut buf,
                            "this._mem",
                            int_ty,
                            format_args!("{0}_ptr", arg_name),
                            format_args!("{0}_i", arg_name),
                        )?;
                        write!(buf, ";\n")?;
                    }
                    write!(buf, "}}\n")?;
                }
            }
        }
        SupportedArgumentType::IntegerSliceRef(_)
        | SupportedArgumentType::IntegerVec(_)
        | SupportedArgumentType::Integer(_) => {}
    }

    Ok(())
}

fn read_three_usize_array<T, U, V, W, X, Y>(
    config: &Config,
    buf: &mut T,
    ptr_name: U,
    temp_prefix: V,
    item1: W,
    item2: X,
    item3: Y,
) -> fmt::Result
where
    T: Write,
    U: Display,
    V: Display,
    W: Display,
    X: Display,
    Y: Display,
{
    match config.access_style {
        AccessStyle::TypedArrays => {
            write!(
                buf,
                r#"let {0}_view = new {2}(this._mem.buffer, {3}, {1});
let {4} = {0}_view[0];
let {5} = {0}_view[1];
let {6} = {0}_view[2];
"#,
                temp_prefix,
                3 * SupportedCopyTy::USize.size_in_bytes(),
                javascript_typed_array_for_int(SupportedCopyTy::USize),
                ptr_name,
                item1,
                item2,
                item3
            )?;
        }
        AccessStyle::DataView => {
            write!(
                buf,
                r#"
let {3} = this._mem.getUint32({0}, true);
let {4} = this._mem.getUint32({0} + {1}, true);
let {5} = this._mem.getUint32({0} + {2}, true);
"#,
                ptr_name,
                SupportedCopyTy::USize.size_in_bytes(),
                SupportedCopyTy::USize.size_in_bytes() * 2,
                item1,
                item2,
                item3
            )?;
        }
    }

    Ok(())
}
fn dealloc_three_usize_array<T, U>(_config: &Config, buf: &mut T, ptr_name: U) -> fmt::Result
where
    T: Write,
    U: Display,
{
    write!(
        buf,
        "this._dealloc({0}, {1});\n",
        ptr_name,
        SupportedCopyTy::USize.size_in_bytes() * 3
    )
}

fn copy_array_out<T, U, V, W, X, Y>(
    config: &Config,
    buf: &mut T,
    ptr_name: U,
    length_name: V,
    byte_length_name: W,
    temp_name: X,
    result_name: Y,
    int_ty: SupportedCopyTy,
) -> fmt::Result
where
    T: Write,
    U: Display,
    V: Display,
    W: Display,
    X: Display,
    Y: Display,
{
    match (config.access_style, int_ty) {
        (AccessStyle::TypedArrays, SupportedCopyTy::Bool) => {
            write!(
                buf,
                r#"let {0}_view = new {3}(this._mem.buffer, return_ptr, {4});
let {2} = [];
for (var {0}_i = 0; {0}_i < {1}; {0}_i++) {{
"#,
                temp_name,
                length_name,
                result_name,
                javascript_typed_array_for_int(int_ty),
                byte_length_name,
            )?;
            write!(
                buf.indented(config.indent),
                "{1}.push(Boolean({0}_view[{0}_i]));\n",
                temp_name,
                result_name,
            )?;
            write!(buf, r"}}\n")?;
        }
        (AccessStyle::TypedArrays, _) => {
            write!(
                buf,
                r#"let {0} = {3}.from(new {3}(this._mem.buffer, {1}, {2}));
"#,
                result_name,
                ptr_name,
                byte_length_name,
                javascript_typed_array_for_int(int_ty)
            )?;
        }
        (AccessStyle::DataView, _) => {
            write!(
                buf,
                r#"let {0} = [];
for (var {1}_i = 0; {1}_i < {2}; {1}_i++) {{
"#,
                result_name,
                temp_name,
                length_name,
            )?;
            {
                let mut buf = buf.indented(config.indent);
                write!(buf, "{0}.push(", result_name)?;
                js_get_ith_ty_at(
                    &mut buf,
                    "this._mem",
                    int_ty,
                    ptr_name,
                    format_args!("{}_i", temp_name),
                )?;
                write!(buf, ");\n")?;
            }
            write!(buf, "}}\n")?;
        }
    }
    Ok(())
}

fn read_return_value_copy_into<T, U, V>(
    config: &Config,
    buf: &mut T,
    ty: &SupportedRetType,
    from_var: U,
    to_var: V,
) -> fmt::Result
where
    T: Write,
    U: Display,
    V: Display,
{
    match *ty {
        SupportedRetType::Unit => {
            write!(buf, "let {} = {};\n", to_var, from_var)?;
        }
        SupportedRetType::Integer(SupportedCopyTy::Bool) => {
            write!(buf, "let {} = Boolean({});\n", to_var, from_var)?;
        }
        SupportedRetType::Integer(_) => {
            write!(buf, "let {} = {};\n", to_var, from_var)?;
        }
        SupportedRetType::IntegerVec(int_ty) => {
            read_three_usize_array(
                config,
                buf,
                "result",
                "result_temp",
                "return_ptr",
                "return_len",
                "return_cap",
            )?;
            write!(
                buf,
                r#"let return_byte_len = return_len * {0};
let return_byte_cap = return_cap * {0};
"#,
                int_ty.size_in_bytes()
            )?;
            copy_array_out(
                config,
                buf,
                "return_ptr",
                "return_len",
                "return_byte_len",
                "return_tmp",
                to_var,
                int_ty,
            )?;
        }
    }

    Ok(())
}

fn deallocate_return_allocation<T, U>(
    config: &Config,
    buf: &mut T,
    from_var: U,
    ty: &SupportedRetType,
) -> fmt::Result
where
    T: Write,
    U: Display,
{
    match *ty {
        SupportedRetType::Unit | SupportedRetType::Integer(_) => {}
        SupportedRetType::IntegerVec(_) => {
            write!(buf, "this._dealloc(return_ptr, return_byte_cap);\n")?;
            dealloc_three_usize_array(config, buf, from_var)?;
        }
    }

    Ok(())
}

fn write_method<T>(
    config: &Config,
    buf: &mut T,
    info: &JsFnInfo,
    stats: &FuncStats,
) -> Result<(), Error>
where
    T: Write,
{
    write!(buf, "\n{}(", info.rust_name)?;
    let mut first_iteration = true;
    for i in 0..info.args_ty.len() {
        if !first_iteration {
            write!(buf, ", ")?;
        }
        write!(buf, "arg{}", i)?;
        first_iteration = false;
    }
    write!(buf, ") {{\n")?;

    {
        let buf = &mut buf.indented(config.indent);
        // argument testing
        for (i, &ty) in info.args_ty.iter().enumerate() {
            validate_argument(
                config,
                buf,
                format_args!("arg{}", i),
                ty,
                "throw new Error();",
            )?;
        }
        // allocation
        for (i, &ty) in info.args_ty.iter().enumerate() {
            prepare_argument_allocation(config, buf, format_args!("arg{}", i), ty)?;
        }

        // actual function call
        write!(buf, "let result = this._funcs['{}'](", info.rust_name)?;
        let mut first_iteration = true;
        for (i, &ty) in info.args_ty.iter().enumerate() {
            if !first_iteration {
                write!(buf, ", ")?;
            }

            match ty {
                SupportedArgumentType::IntegerSliceRef(_)
                | SupportedArgumentType::IntegerSliceMutRef(_)
                | SupportedArgumentType::IntegerVec(_) => {
                    write!(buf, "arg{0}_ptr, arg{0}_len", i)?;
                }
                SupportedArgumentType::Integer(_) => {
                    write!(buf, "arg{0}", i)?;
                }
            }
            first_iteration = false;
        }

        write!(buf, ");\n")?;
        if config.access_style == AccessStyle::DataView && stats.uses_post_function_memory_access {
            write!(buf, "this._check_mem_realloc();\n")?;
        }

        for (i, &ty) in info.args_ty.iter().enumerate() {
            propogate_argument_changes_outwards(config, buf, format_args!("arg{}", i), ty)?;
        }

        read_return_value_copy_into(config, buf, &info.ret_ty, "result", "return_value")?;

        for (i, &ty) in info.args_ty.iter().enumerate() {
            deallocate_argument_allocation(config, buf, format_args!("arg{}", i), ty)?;
        }

        deallocate_return_allocation(config, buf, "result", &info.ret_ty)?;

        write!(buf, "return return_value;\n")?;
    }

    write!(buf, "}}\n")?;

    Ok(())
}


fn write_class_definition_finish<T>(config: &Config, buf: &mut T) -> Result<(), Error>
where
    T: Write,
{
    write!(
        buf,
        r#"}}

exports = module.exports = {};
"#,
        config.class_name,
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

fn javascript_typed_array_for_int(ty: SupportedCopyTy) -> &'static str {
    use self::SupportedCopyTy::*;
    match ty {
        U8 => "Uint8Array",
        U16 => "Uint16Array",
        U32 => "Uint32Array",
        I8 => "Int8Array",
        I16 => "Int16Array",
        I32 => "Int32Array",
        USize => "Uint32Array",
        ISize => "Int32Array",
        F32 => "Float32Array",
        F64 => "Float64Array",
        Bool => "Uint8Array", // additional code needed to handle this case
    }
}

fn js_set_ith_ty_at<T, U, V, W, X>(
    mut buf: T,
    data_view_name: U,
    ty: SupportedCopyTy,
    ptr_name: V,
    i_name: W,
    value: X,
) -> Result<(), fmt::Error>
where
    T: fmt::Write,
    U: Display,
    V: Display,
    W: Display,
    X: Display,
{
    use self::SupportedCopyTy::*;

    let value = match ty {
        Bool => format!("Boolean({})", value),
        _ => value.to_string(),
    };

    let set_func_name = match ty {
        Bool | U8 => "setUint8",
        U16 => "setUint16",
        USize | U32 => "setUint32",
        I8 => "setInt8",
        I16 => "setInt16",
        ISize | I32 => "setInt32",
        F32 => "setFloat32",
        F64 => "setFloat64",
    };

    let offset = ty.size_in_bytes();

    write!(
        buf,
        "{}.{}({} + {} * {}, {}, true);\n",
        data_view_name,
        set_func_name,
        ptr_name,
        offset,
        i_name,
        value
    )
}

fn js_get_ith_ty_at<T, U, V, W>(
    mut buf: T,
    data_view_name: U,
    ty: SupportedCopyTy,
    ptr_name: V,
    i_name: W,
) -> Result<(), fmt::Error>
where
    T: fmt::Write,
    U: Display,
    V: Display,
    W: Display,
{
    use self::SupportedCopyTy::*;

    let get_func_name = match ty {
        Bool | U8 => "getUint8",
        U16 => "getUint16",
        USize | U32 => "getUint32",
        I8 => "getInt8",
        I16 => "getInt16",
        ISize | I32 => "getInt32",
        F32 => "getFloat32",
        F64 => "getFloat64",
    };

    let offset = ty.size_in_bytes();

    match ty {
        Bool => write!(
            buf,
            "Boolean({}.{}({} + {} * {}, true))",
            data_view_name,
            get_func_name,
            ptr_name,
            offset,
            i_name
        ),
        _ => write!(
            buf,
            "{}.{}({} + {} * {}, true)",
            data_view_name,
            get_func_name,
            ptr_name,
            offset,
            i_name
        ),
    }
}
