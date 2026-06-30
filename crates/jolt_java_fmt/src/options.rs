use jolt_fmt_ir::RenderOptions;

/// Java formatter options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaFormatOptions {
    /// Compatibility profile that selects Java-specific formatting policies.
    pub profile: JavaFormatProfile,
    /// Language-neutral rendering options used by the Java formatter.
    pub render: RenderOptions,
}

impl Default for JavaFormatOptions {
    fn default() -> Self {
        Self::for_profile(JavaFormatProfile::Google)
    }
}

impl JavaFormatOptions {
    /// Returns concrete Java formatter options for a compatibility profile.
    #[must_use]
    pub fn for_profile(profile: JavaFormatProfile) -> Self {
        let render = match profile {
            JavaFormatProfile::Google => RenderOptions::default(),
            JavaFormatProfile::Aosp | JavaFormatProfile::Palantir => RenderOptions {
                indent_width: 4,
                ..RenderOptions::default()
            },
        };

        Self { profile, render }
    }
}

/// Java formatter compatibility profile convenience selector.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum JavaFormatProfile {
    /// Compatibility target for Google Java Format.
    #[default]
    Google,
    /// Compatibility target for Google Java Format AOSP mode.
    Aosp,
    /// Compatibility target for Palantir Java Format.
    Palantir,
}
