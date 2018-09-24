use std::ffi::{OsString};
use std::ops::Deref;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectoryName(OsString);
impl DirectoryName {
    pub fn new(oss: OsString) -> Result<Self, DirectoryNameError> {
        let s = match oss.into_string() {
            Ok(s) => s,
            Err(oss) => return Err(DirectoryNameError::UnsupportedCharacters(oss))
        };

        if let Some(index) = s.find(|c: char| (c == '/') || (c == '.')) {
            return Err(DirectoryNameError::InvalidCharacterAtIndex(index))
        }

        Ok(DirectoryName(s.into()))
    }
}

impl Deref for DirectoryName {
    type Target = <OsString as Deref>::Target;
    fn deref(&self) -> &Self::Target { self.0.deref() }
}

#[derive(Debug, PartialEq)]
pub enum DirectoryNameError {
    InvalidCharacterAtIndex(usize),
    UnsupportedCharacters(OsString),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_directory_name() {
        assert_eq!(DirectoryName::new("directory1".into()), Ok(DirectoryName("directory1".into())));
        assert_eq!(DirectoryName::new("directory 2".into()), Ok(DirectoryName("directory 2".into())));
        assert_eq!(DirectoryName::new("directory 📦".into()), Ok(DirectoryName("directory 📦".into())));
    }

    #[test]
    fn invalid_character_in_directory_name() {
        assert_eq!(DirectoryName::new("directory/1".into()), Err(DirectoryNameError::InvalidCharacterAtIndex(9)));
        assert_eq!(DirectoryName::new("directory.2".into()), Err(DirectoryNameError::InvalidCharacterAtIndex(9)));
    }

    #[test]
    fn invalid_encoding_in_directory_name() {
        #[cfg(windows)]
        use std::os::windows::ffi::OsStringExt;
        #[cfg(unix)]
        use std::os::unix::ffi::OsStringExt;

        #[cfg(windows)]
        let oss = OsString::from_wide(&[0xEEEE, 0xDDDD, 0xFFFF, 0x0888][..]);
        #[cfg(unix)]
        let oss = OsString::from_vec(&[0xfe, 0xff, 0xfe, 0xfe, 0xff, 0xff][..]);

        assert_eq!(DirectoryName::new(oss.clone()), Err(DirectoryNameError::UnsupportedCharacters(oss)));
    }
}
