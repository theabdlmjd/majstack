macro_rules! str_enum {
    ($name:ident { $($variant:ident => $s:literal),+ $(,)? }) => {
        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $( $name::$variant => $s ),+
                }
            }
            pub fn parse(value: &str) -> Option<Self> {
                match value {
                    $( $s => Some($name::$variant), )+
                    _ => None,
                }
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}
