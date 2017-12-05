use std::borrow::Cow;

// TODO: add hyperlinks to documentation.
#[derive(Clone, Debug)]
pub struct Config<'a> {
    /// Class name to generate. Default "WasmWrapper".
    pub(crate) class_name: Cow<'a, str>,
    /// Number of spaces to indent with. Default 4.
    pub(crate) indent: u32,
    /// Array access style to use. Default DataView.
    pub(crate) access_style: AccessStyle,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AccessStyle {
    /// Construct typed arrays in each function for use.
    ///
    /// This has the disadvantage of more objects, but the advantage of being
    /// able to use TypedArray.set(), a function for setting the entire array
    /// to values provided. I have no idea if it's better at all, so profile
    /// your use case!
    TypedArrays,
    /// Construct a single DataView for the module and use its methods in each
    /// function to set individual values of arrays.
    ///
    /// This is the default.
    DataView,
}

impl Default for AccessStyle {
    fn default() -> Self {
        AccessStyle::DataView
    }
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        Config {
            class_name: "WasmWrapper".into(),
            indent: 4,
            access_style: AccessStyle::default(),
        }
    }
}

impl<'a> Config<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_class_name<T: Into<Cow<'a, str>>>(&mut self, class_name: T) -> &mut Self {
        self.class_name = class_name.into();
        self
    }

    pub fn with_indent(&mut self, indent: u32) -> &mut Self {
        self.indent = indent;
        self
    }

    pub fn with_array_access_style(&mut self, style: AccessStyle) -> &mut Self {
        self.access_style = style;
        self
    }
}
