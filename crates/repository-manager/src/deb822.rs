/// DEB822 format source entry.
///
/// Represents a single stanza in a `.sources` file under
/// `/etc/apt/sources.list.d/`. This is the modern apt repository format
/// used by Debian 12+ and preferred over the legacy one-line `deb` format.
use std::fmt;

/// A single DEB822 source entry (one stanza).
#[derive(Debug, Clone)]
pub struct Deb822Source {
    /// Package types: "deb", "deb-src", or both
    pub types: Vec<String>,
    /// Repository URIs
    pub uris: Vec<String>,
    /// Distribution suites (e.g. "trixie", "trixie-updates")
    pub suites: Vec<String>,
    /// Repository components (e.g. "main", "contrib")
    pub components: Vec<String>,
    /// Path to the signed-by keyring (optional)
    pub signed_by: Option<String>,
    /// Architectures (optional)
    pub architectures: Option<Vec<String>>,
}

impl fmt::Display for Deb822Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Types
        writeln!(f, "Types: {}", self.types.join(" "))?;

        // URIs
        writeln!(f, "URIs: {}", self.uris.join(" "))?;

        // Suites
        writeln!(f, "Suites: {}", self.suites.join(" "))?;

        // Components (if present)
        if !self.components.is_empty() {
            writeln!(f, "Components: {}", self.components.join(" "))?;
        }

        // Architectures (if specified)
        if let Some(ref archs) = self.architectures {
            writeln!(f, "Architectures: {}", archs.join(" "))?;
        }

        // Signed-By (if specified)
        if let Some(ref key) = self.signed_by {
            writeln!(f, "Signed-By: {}", key)?;
        }

        Ok(())
    }
}

/// A complete `.sources` file containing one or more stanzas.
#[derive(Debug, Clone)]
pub struct DebianSourceEntry {
    pub entries: Vec<Deb822Source>,
}

impl DebianSourceEntry {
    pub fn new() -> Self {
        DebianSourceEntry { entries: Vec::new() }
    }

    pub fn add(&mut self, entry: Deb822Source) {
        self.entries.push(entry);
    }
}

impl Default for DebianSourceEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DebianSourceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", entry)?;
        }
        Ok(())
    }
}

/// Build the default Debian repository configuration for Trixie.
pub fn default_debian_sources() -> DebianSourceEntry {
    let mut entry = DebianSourceEntry::new();

    // Main Debian repository
    entry.add(Deb822Source {
        types: vec!["deb".into()],
        uris: vec!["https://mirrors.tuna.tsinghua.edu.cn/debian".into()],
        suites: vec![
            "trixie".into(),
            "trixie-updates".into(),
            "trixie-backports".into(),
        ],
        components: vec![
            "main".into(),
            "contrib".into(),
            "non-free".into(),
            "non-free-firmware".into(),
        ],
        signed_by: Some("/usr/share/keyrings/debian-archive-keyring.gpg".into()),
        architectures: None,
    });

    // Security repository
    entry.add(Deb822Source {
        types: vec!["deb".into()],
        uris: vec!["https://mirrors.tuna.tsinghua.edu.cn/debian-security".into()],
        suites: vec!["trixie-security".into()],
        components: vec![
            "main".into(),
            "contrib".into(),
            "non-free".into(),
            "non-free-firmware".into(),
        ],
        signed_by: Some("/usr/share/keyrings/debian-archive-keyring.gpg".into()),
        architectures: None,
    });

    entry
}

/// Build a Lingmo OBS repository source entry.
pub fn lingmo_obs_source(key_path: &str) -> Deb822Source {
    Deb822Source {
        types: vec!["deb".into()],
        uris: vec![
            "http://download.opensuse.org/repositories/home:/LingmoOS/Debian_13/".into(),
        ],
        suites: vec!["/".into()],
        components: vec![],
        signed_by: Some(key_path.into()),
        architectures: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deb822_format() {
        let entry = Deb822Source {
            types: vec!["deb".into()],
            uris: vec!["https://deb.debian.org/debian".into()],
            suites: vec!["bookworm".into(), "bookworm-updates".into()],
            components: vec!["main".into(), "contrib".into()],
            signed_by: Some("/usr/share/keyrings/debian-archive-keyring.gpg".into()),
            architectures: None,
        };

        let output = entry.to_string();
        assert!(output.contains("Types: deb"));
        assert!(output.contains("URIs: https://deb.debian.org/debian"));
        assert!(output.contains("Suites: bookworm bookworm-updates"));
        assert!(output.contains("Components: main contrib"));
        assert!(output.contains("Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg"));
    }

    #[test]
    fn test_multi_stanza() {
        let mut file = DebianSourceEntry::new();
        file.add(Deb822Source {
            types: vec!["deb".into()],
            uris: vec!["https://deb.debian.org/debian".into()],
            suites: vec!["bookworm".into()],
            components: vec!["main".into()],
            signed_by: None,
            architectures: None,
        });
        file.add(Deb822Source {
            types: vec!["deb".into()],
            uris: vec!["https://security.debian.org".into()],
            suites: vec!["bookworm-security".into()],
            components: vec!["main".into()],
            signed_by: None,
            architectures: None,
        });

        let output = file.to_string();
        assert!(output.contains("Types: deb"));
        // Should have two stanzas separated by blank line
        assert_eq!(output.matches("Types: deb").count(), 2);
    }
}
