use bitflags::bitflags;
use ctru::services::fs::ArchiveID;
use ctru::services::fs::MediaType;
use ctru::services::fs::PathType;
use ctru_sys::PATH_BINARY;
use log::error;
use std::ffi::c_void;
use std::ffi::OsString;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;
use std::io::Result as IoResult;
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem;
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::sync::Arc;
use widestring::{WideCStr, WideCString};

bitflags! {
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    struct FsOpen: u32 {
        const FS_OPEN_READ   = ctru_sys::FS_OPEN_READ;
        const FS_OPEN_WRITE  = ctru_sys::FS_OPEN_WRITE;
        const FS_OPEN_CREATE = ctru_sys::FS_OPEN_CREATE;
    }

    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    struct FsWrite: u32 {
        const FS_WRITE_FLUSH       = ctru_sys::FS_WRITE_FLUSH;
        const FS_WRITE_UPDATE_TIME = ctru_sys::FS_WRITE_UPDATE_TIME;
    }

    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    struct FsAttribute: u32 {
        const FS_ATTRIBUTE_DIRECTORY = ctru_sys::FS_ATTRIBUTE_DIRECTORY;
        const FS_ATTRIBUTE_HIDDEN    = ctru_sys::FS_ATTRIBUTE_HIDDEN;
        const FS_ATTRIBUTE_ARCHIVE   = ctru_sys::FS_ATTRIBUTE_ARCHIVE;
        const FS_ATTRIBUTE_READ_ONLY = ctru_sys::FS_ATTRIBUTE_READ_ONLY;
    }
}

/// Handle to an open filesystem archive.
///
/// Archives are automatically closed when they go out of scope.
///
/// # Examples
///
/// ```
/// # let _runner = test_runner::GdbRunner::default();
/// use ctru::services::fs::Fs;
///
/// let mut fs = Fs::new().unwrap();
/// let sdmc_archive = fs.sdmc().unwrap();
/// ```
pub struct Archive {
    pub id: ArchiveID,
    pub handle: u64,
}

/// A reference to an open file on the filesystem.
///
/// An instance of a `File` can be read and/or written to depending
/// on what options It was opened with.
///
/// Files are automatically closed when they go out of scope.
///
/// # Examples
///
/// Create a new file and write bytes to it:
///
/// ```
/// # let _runner = test_runner::GdbRunner::default();
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #
/// use std::io::prelude::*;
///
/// use ctru::services::fs::{File, Fs};
///
/// let mut fs = Fs::new()?;
/// let mut sdmc = fs.sdmc()?;
/// #
/// # Ok(())
/// # }
/// ```
///
/// Read the contents of a file into a `String`::
///
/// ```
/// # let _runner = test_runner::GdbRunner::default();
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #
/// use std::io::prelude::*;
///
/// use ctru::services::fs::{File, Fs};
///
/// let mut fs = Fs::new()?;
/// let mut sdmc = fs.sdmc()?;
///
/// let mut file = File::open(&sdmc, "/foo.txt")?;
/// let mut contents = String::new();
/// file.read_to_string(&mut contents)?;
/// assert_eq!(contents, "Hello, world!");
/// #
/// # Ok(())
/// # }
/// ```
///
/// It can be more efficient to read the contents of a file with a buffered
/// `Read`er. This can be accomplished with `BufReader<R>`:
///
/// ```
/// # let _runner = test_runner::GdbRunner::default();
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #
/// use std::io::prelude::*;
/// use std::io::BufReader;
///
/// use ctru::services::fs::{File, Fs};
///
/// let mut fs = Fs::new()?;
/// let mut sdmc = fs.sdmc()?;
///
/// let file = File::open(&sdmc, "/foo.txt")?;
/// let mut buf_reader = BufReader::new(file);
/// let mut contents = String::new();
/// buf_reader.read_to_string(&mut contents)?;
/// assert_eq!(contents, "Hello, world!");
/// #
/// # Ok(())
/// # }
/// ```
pub struct File {
    handle: u32,
    offset: u64,
}

/// Metadata information about a file.
///
/// This structure is returned from the [`File::metadata`] function and
/// represents known metadata about a file.
pub struct Metadata {
    attributes: u32,
    size: u64,
}

/// Options and flags which can be used to configure how a [`File`] is opened.
/// This builder exposes the ability to configure how a [`File`] is opened
/// and what operations are permitted on the open file. The [`File::open`]
/// and [`File::create`] methods are aliases for commonly used options
/// using this builder.
///
/// Generally speaking, when using [`OpenOptions`], you'll first call [`OpenOptions::new`],
/// then chain calls to methods to set each option, then call [`OpenOptions::open`],
/// passing the path of the file you're trying to open.
///
/// It is required to also pass a reference to the [`Archive`] that the
/// file lives in.
///
/// # Examples
///
/// Opening a file to read:
///
/// ```
/// # let _runner = test_runner::GdbRunner::default();
/// use ctru::services::fs::{Fs, OpenOptions};
///
/// let mut fs = Fs::new().unwrap();
/// let mut sdmc_archive = fs.sdmc().unwrap();
/// let result = OpenOptions::new()
///             .read(true)
///             .archive(&sdmc_archive)
///     .open("foo.txt");
///
/// assert!(result.is_err());
/// ```
///
/// Opening a file for both reading and writing, as well as creating it if it
/// doesn't exist:
///
/// ```
/// # let _runner = test_runner::GdbRunner::default();
/// use ctru::services::fs::{Fs, OpenOptions};
///
/// let mut fs = Fs::new().unwrap();
/// let mut sdmc_archive = fs.sdmc().unwrap();
/// let file = OpenOptions::new()
///             .read(true)
///             .write(true)
///             .create(true)
///             .archive(&sdmc_archive)
///     .open("/foo.txt")
///             .unwrap();
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenOptions {
    input_size: Option<u64>,
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    arch_handle: u64,
}

fn extdata_redirect(low_id: u32) -> u32 {
    match low_id {
        // Pokemon Y
        0x00055E00 => 0x0000055D,
        // Pokemon OR
        0x0011C400 => 0x000011C5,
        // Pkmn moon
        0x00175E00 => 0x00001648,
        // Ultra moon
        0x001B5100 => 0x00001B50,
        // FE Conquest + SE NA
        0x00179600 | 0x00179800 => 0x00001794,
        // FE Conquest + SE Euro
        0x00179700 | 0x0017A800 => 0x00001795,
        // FE if JP
        0x0012DD00 | 0x0012DE00 => 0x000012DC,
        _ => low_id >> 8,
    }
}

/// Iterator over the entries in a directory.
///
/// This iterator is returned from the [`read_dir`] function and
/// will yield instances of [`Result<DirEntry, i32>`]. Through a [`DirEntry`]
/// information like the entry's path and possibly other metadata can be
/// learned.
///
/// # Errors
///
/// This Result will return Err if there's some sort of intermittent IO error
/// during iteration.
pub struct ReadDir<'a> {
    handle: Dir,
    root: Arc<PathBuf>,
    arch: &'a Archive,
}

/// Entries returned by the [`ReadDir`] iterator.
///
/// An instance of `DirEntry` represents an entry inside of a directory on the
/// filesystem. Each entry can be inspected via methods to learn about the full
/// path or possibly other metadata.
pub struct DirEntry<'a> {
    entry: ctru_sys::FS_DirectoryEntry,
    root: Arc<PathBuf>,
    arch: &'a Archive,
}

#[doc(hidden)]
struct Dir(u32);

#[doc(hidden)]
unsafe impl Send for Dir {}
#[doc(hidden)]
unsafe impl Sync for Dir {}

pub fn arch(
    arch_id: ArchiveID,
    media: MediaType,
    high_id: u32,
    low_id: u32,
) -> ctru::Result<Archive> {
    let ext_data = extdata_redirect(low_id);
    unsafe {
        let mut handle = 0;
        if let Some(r) = match arch_id {
            ArchiveID::Sdmc => {
                let path = ctru_sys::fsMakePath(PathType::Empty.into(), ptr::null() as _);
                Some(ctru_sys::FSUSER_OpenArchive(
                    &mut handle,
                    arch_id.into(),
                    path,
                ))
            }
            ArchiveID::UserSavedata => {
                let path_data: [u32; 3] = [media.into(), low_id, high_id];
                let path = ctru_sys::FS_Path {
                    type_: PATH_BINARY,
                    size: 12,
                    data: path_data.as_ptr() as *const c_void,
                };
                Some(ctru_sys::FSUSER_OpenArchive(
                    &mut handle,
                    arch_id.into(),
                    path,
                ))
            }
            ArchiveID::Savedata => {
                let path = ctru_sys::fsMakePath(PathType::Empty.into(), ptr::null() as _);
                Some(ctru_sys::FSUSER_OpenArchive(
                    &mut handle,
                    arch_id.into(),
                    path,
                ))
            }
            ArchiveID::Extdata | ArchiveID::BossExtdata => {
                let path_data: [u32; 3] = [MediaType::Sd.into(), ext_data, 0];
                let path = ctru_sys::FS_Path {
                    type_: PATH_BINARY,
                    size: 12,
                    data: path_data.as_ptr() as *const c_void,
                };
                Some(ctru_sys::FSUSER_OpenArchive(
                    &mut handle,
                    arch_id.into(),
                    path,
                ))
            }
            ArchiveID::SystemSavedata => {
                let path_data: [u32; 2] = [MediaType::Nand.into(), (0x00020000 | (low_id >> 8))];
                let path = ctru_sys::FS_Path {
                    type_: PATH_BINARY,
                    size: 8,
                    data: path_data.as_ptr() as *const c_void,
                };
                Some(ctru_sys::FSUSER_OpenArchive(
                    &mut handle,
                    arch_id.into(),
                    path,
                ))
            }
            ArchiveID::SharedExtdata => {
                let path_data: [u32; 3] = [MediaType::Nand.into(), ext_data, 0x00048000];
                let path = ctru_sys::FS_Path {
                    type_: PATH_BINARY,
                    size: 0xC,
                    data: path_data.as_ptr() as *const c_void,
                };
                Some(ctru_sys::FSUSER_OpenArchive(
                    &mut handle,
                    arch_id.into(),
                    path,
                ))
            }
            _ => None,
        } {
            if r < 0 {
                Err(ctru::Error::from(r))
            } else {
                Ok(Archive {
                    handle,
                    id: arch_id,
                })
            }
        } else {
            Err(ctru::Error::from(-1))
        }
    }
}

impl Archive {
    /// Retrieves an Archive's [`ArchiveID`]
    pub fn id(&self) -> ArchiveID {
        self.id
    }
}

impl File {
    /// Attempts to open a file in read-only mode.
    ///
    /// See the [`OpenOptions::open`] method for more details.
    ///
    /// # Errors
    ///
    /// This function will return an error if `path` does not already exit.
    /// Other errors may also be returned accoridng to [`OpenOptions::open`]
    ///
    /// # Examples
    ///
    /// ```
    /// # let _runner = test_runner::GdbRunner::default();
    /// use ctru::services::fs::{File, Fs};
    ///
    /// let mut fs =  Fs::new().unwrap();
    /// let mut sdmc_archive = fs.sdmc().unwrap();
    /// // Non-existent file:
    /// assert!(File::open(&sdmc_archive, "/foo.txt").is_err());
    /// ```
    pub fn open<P: AsRef<Path>>(arch: &Archive, path: P) -> IoResult<File> {
        OpenOptions::new()
            .read(true)
            .archive(arch)
            .open(path.as_ref())
    }

    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist.
    ///
    /// See the [`OpenOptions::create`] method for more details.
    ///
    /// # Errors
    ///
    /// This function will return an error if `path` does not already exit.
    /// Other errors may also be returned accoridng to [`OpenOptions::create`].
    ///
    /// # Examples
    ///
    /// ```
    /// # let _runner = test_runner::GdbRunner::default();
    /// use ctru::services::fs::{File, Fs};
    ///
    /// let mut fs =  Fs::new().unwrap();
    /// let mut sdmc_archive = fs.sdmc().unwrap();
    /// let mut f = File::create(&mut sdmc_archive, "/foo.txt").unwrap();
    /// ```
    pub fn create<P: AsRef<Path>>(
        arch: &Archive,
        path: P,
        input_size: Option<u64>,
    ) -> IoResult<File> {
        OpenOptions::new()
            .input_size(input_size)
            .write(true)
            .create(true)
            .archive(arch)
            .open(path.as_ref())
    }

    /// Truncates or extends the underlying file, updating the size of this file to become size.
    ///
    /// If the size is less than the current file's size, then the file will be shrunk. If it is
    /// greater than the current file's size, then the file will be extended to size and have all
    /// of the intermediate data filled in with 0s.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file is not opened for writing.
    #[doc(alias = "FSFILE_SetSize")]
    pub fn set_len(&mut self, size: u64) -> IoResult<()> {
        unsafe {
            let r = ctru_sys::FSFILE_SetSize(self.handle, size);
            if r < 0 {
                Err(IoError::new(
                    IoErrorKind::PermissionDenied,
                    ctru::Error::from(r),
                ))
            } else {
                Ok(())
            }
        }
    }

    /// Queries metadata about the underlying file.
    pub fn metadata(&self) -> IoResult<Metadata> {
        // The only metadata we have for files right now is file size.
        // This is likely to change in the future.
        unsafe {
            let mut size = 0;
            let r = ctru_sys::FSFILE_GetSize(self.handle, &mut size);
            if r < 0 {
                Err(IoError::new(
                    IoErrorKind::PermissionDenied,
                    ctru::Error::from(r),
                ))
            } else {
                Ok(Metadata {
                    attributes: 0,
                    size,
                })
            }
        }
    }

    #[doc(alias = "FSFILE_Read")]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        unsafe {
            let mut n_read = 0;
            let r = ctru_sys::FSFILE_Read(
                self.handle,
                &mut n_read,
                self.offset,
                buf.as_mut_ptr() as _,
                buf.len() as u32,
            );
            if r < 0 {
                if n_read > buf.len() as u32 {
                    n_read = buf.len() as u32;
                }
                error!("Error: {}", ctru::Error::from(r));
                // return Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)))
            }
            self.offset += n_read as u64;
            Ok(n_read as usize)
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> IoResult<usize> {
        unsafe { read_to_end_uninitialized(self, buf) }
    }

    #[doc(alias = "FSFILE_Write")]
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        unsafe {
            let mut n_written = 0;
            let r = ctru_sys::FSFILE_Write(
                self.handle,
                &mut n_written,
                self.offset,
                buf.as_ptr() as _,
                buf.len() as u32,
                FsWrite::FS_WRITE_UPDATE_TIME.bits(),
            );
            self.offset += n_written as u64;
            if r < 0 {
                Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)))
            } else {
                Ok(n_written as usize)
            }
        }
    }
}

impl Metadata {
    /// Returns whether this metadata is for a directory.
    pub fn is_dir(&self) -> bool {
        self.attributes == self.attributes | FsAttribute::FS_ATTRIBUTE_DIRECTORY.bits()
    }

    /// Returns whether this metadata is for a regular file.
    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }

    /// Returns the size, in bytes, this metadata is for.
    ///
    /// Directories return size = 0.
    // We don't want an is_empty function because directories always have a
    // zero size.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u64 {
        self.size
    }
}

impl OpenOptions {
    /// Creates a blank set of options ready for configuration.
    ///
    /// All options are initially set to `false`
    pub fn new() -> OpenOptions {
        Self::default()
    }

    /// Set the option for read access.
    ///
    /// This option, when true, will indicate that the file should be
    /// `read`-able if opened.
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.read = read;
        self
    }

    pub fn input_size(&mut self, input_size: Option<u64>) -> &mut OpenOptions {
        self.input_size = input_size;
        self
    }

    /// Set the option for write access.
    ///
    /// This option, when true, will indicate that the file should be
    /// `write`-able if opened.
    ///
    /// If the file already exists, any write calls on it will overwrite
    /// its contents, without truncating it.
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.write = write;
        self
    }

    /// Set the option for the append mode.
    ///
    /// This option, when true, means that writes will append to a file instead
    /// of overwriting previous contents. Note that setting .write(true).append(true)
    /// has the same effect as setting only .append(true).
    ///
    /// If both truncate and append are set to true, the file will simply be truncated
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.append = append;
        self
    }

    /// Set the option for truncating a previous file.
    ///
    /// If a file is successfully opened with this option set it will truncate
    /// the file to 0 length if it already exists.
    ///
    /// The file must be opened with write access for truncate to work.
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.truncate = truncate;
        self
    }

    /// Set the option for creating a new file.
    ///
    /// This option indicates whether a new file will be created
    /// if the file does not yet already
    /// exist.
    ///
    /// In order for the file to be created, write access must also be used.
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.create = create;
        self
    }

    /// Set which archive the file is to be opened in.
    ///
    /// Failing to pass in an archive will result in the file failing to open.
    pub fn archive(&mut self, archive: &Archive) -> &mut OpenOptions {
        self.arch_handle = archive.handle;
        self
    }

    /// Opens a file at `path` with the options specified by `self`
    ///
    /// # Errors
    ///
    /// This function will return an error under a number of different
    /// circumstances, including but not limited to:
    ///
    /// * Opening a file that doesn't exist without setting `create`.
    /// * Attempting to open a file without passing an [`Archive`] reference
    ///   to the `archive` method.
    /// * Filesystem-level errors (full disk, etc).
    /// * Invalid combinations of open options.
    #[doc(alias = "FSUSER_OpenFile")]
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> IoResult<File> {
        self._open(path.as_ref(), self.open_flags())
    }

    fn _open(&mut self, path: &Path, flags: FsOpen) -> IoResult<File> {
        unsafe {
            let mut file_handle = 0;
            let path = to_utf16(path);
            let fs_path = ctru_sys::fsMakePath(PathType::UTF16.into(), path.as_ptr() as _);
            let mut r = ctru_sys::FSUSER_OpenFile(
                &mut file_handle,
                self.arch_handle,
                fs_path,
                flags.bits(),
                0,
            );
            if r < 0 {
                if self.create && self.input_size.is_some() {
                    // create file
                    r = ctru_sys::FSUSER_CreateFile(
                        self.arch_handle,
                        fs_path,
                        0,
                        self.input_size.unwrap(),
                    );

                    // open file
                    if r >= 0 {
                        r = ctru_sys::FSUSER_OpenFile(
                            &mut file_handle,
                            self.arch_handle,
                            fs_path,
                            FsOpen::FS_OPEN_WRITE.bits(),
                            0,
                        );
                    }
                }
                if r < 0 {
                    return Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)));
                }
            }

            let mut file = File {
                handle: file_handle,
                offset: 0,
            };

            if self.append {
                let metadata = file.metadata()?;
                file.offset = metadata.len();
            }

            // set the offset to 0 just in case both append and truncate were
            // set to true
            if self.truncate {
                file.set_len(0)?;
                file.offset = 0;
            }
            Ok(file)
        }
    }

    fn open_flags(&self) -> FsOpen {
        match (self.read, self.write || self.append, self.create) {
            (true, false, false) => FsOpen::FS_OPEN_READ,
            (false, true, false) => FsOpen::FS_OPEN_WRITE,
            (false, true, true) => FsOpen::FS_OPEN_WRITE | FsOpen::FS_OPEN_CREATE,
            (true, false, true) => FsOpen::FS_OPEN_READ | FsOpen::FS_OPEN_CREATE,
            (true, true, false) => FsOpen::FS_OPEN_READ | FsOpen::FS_OPEN_WRITE,
            (true, true, true) => {
                FsOpen::FS_OPEN_READ | FsOpen::FS_OPEN_WRITE | FsOpen::FS_OPEN_CREATE
            }
            _ => FsOpen::empty(), //failure case
        }
    }
}

impl<'a> Iterator for ReadDir<'a> {
    type Item = IoResult<DirEntry<'a>>;

    fn next(&mut self) -> Option<IoResult<DirEntry<'a>>> {
        unsafe {
            let mut ret = DirEntry {
                entry: mem::zeroed(),
                root: self.root.clone(),
                arch: self.arch,
            };
            let mut entries_read = 0;
            let entry_count = 1;
            let r = ctru_sys::FSDIR_Read(
                self.handle.0,
                &mut entries_read,
                entry_count,
                &mut ret.entry,
            );

            if r < 0 {
                return Some(Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r))));
            }
            if entries_read != entry_count {
                return None;
            }
            Some(Ok(ret))
        }
    }
}

impl<'a> DirEntry<'a> {
    /// Returns the full path to the file that this entry represents.
    ///
    /// The full path is created by joining the original path to `read_dir`
    /// with the filename of this entry.
    pub fn path(&self) -> PathBuf {
        self.root.join(self.file_name())
    }

    /// Return the metadata for the file that this entry points at.
    pub fn metadata(&self) -> IoResult<Metadata> {
        metadata(self.arch, self.path())
    }

    /// Returns the bare file name of this directory entry without any other leading path
    /// component.
    pub fn file_name(&self) -> OsString {
        unsafe {
            let filename = truncate_utf16_at_nul(&self.entry.name);
            let filename = WideCStr::from_ptr_str(filename.as_ptr());
            filename.to_os_string()
        }
    }
}

/// Creates a new, empty directory at the provided path
///
/// # Errors
///
/// This function will return an error in the following situations,
/// but is not limited to just these cases:
///
/// * User lacks permissions to create directory at `path`
#[doc(alias = "FSUSER_CreateDirectory")]
pub fn create_dir<P: AsRef<Path>>(arch: &Archive, path: P) -> IoResult<()> {
    unsafe {
        let path = to_utf16(path.as_ref());
        let fs_path = ctru_sys::fsMakePath(PathType::UTF16.into(), path.as_ptr() as _);
        let r = ctru_sys::FSUSER_CreateDirectory(
            arch.handle,
            fs_path,
            FsAttribute::FS_ATTRIBUTE_DIRECTORY.bits(),
        );
        if r < 0 {
            Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)))
        } else {
            Ok(())
        }
    }
}

/// Recursively create a directory and all of its parent components if they are missing.
///
/// # Errors
///
/// This function will return an error in the following situations,
/// but is not limited to just these cases:
///
/// * If any directory in the path specified by `path` does not already exist
///   and it could not be created otherwise.
#[doc(alias = "FSUSER_CreateDirectory")]
pub fn create_dir_all<P: AsRef<Path>>(arch: &Archive, path: P) -> IoResult<()> {
    let path = path.as_ref();
    let mut dir = PathBuf::new();
    let mut result = Ok(());

    for component in path.components() {
        let component = component.as_os_str();
        dir.push(component);
        result = create_dir(arch, &dir);
    }
    result
}

pub fn is_dir_exists(arch: &Archive, path: impl AsRef<Path>) -> bool {
    let maybe_dir = read_dir(arch, path.as_ref());
    match maybe_dir {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Given a path, query the file system to get information about a file, directory, etc
pub fn metadata<P: AsRef<Path>>(arch: &Archive, path: P) -> IoResult<Metadata> {
    let maybe_file = File::open(arch, path.as_ref());
    let maybe_dir = read_dir(arch, path.as_ref());
    match (maybe_file, maybe_dir) {
        (Ok(file), _) => file.metadata(),
        (_, Ok(_dir)) => Ok(Metadata {
            attributes: FsAttribute::FS_ATTRIBUTE_DIRECTORY.bits(),
            size: 0,
        }),
        (Err(e), _) => Err(e),
    }
}

/// Removes an existing, empty directory.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not limited to just
/// these cases:
///
/// * The user lacks permissions to remove the directory at the provided path.
/// * The directory isn't empty.
#[doc(alias = "FSUSER_DeleteDirectory")]
pub fn remove_dir<P: AsRef<Path>>(arch: &Archive, path: P) -> IoResult<()> {
    unsafe {
        let path = to_utf16(path.as_ref());
        let fs_path = ctru_sys::fsMakePath(PathType::UTF16.into(), path.as_ptr() as _);
        let r = ctru_sys::FSUSER_DeleteDirectory(arch.handle, fs_path);
        if r < 0 {
            Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)))
        } else {
            Ok(())
        }
    }
}

/// Removes a directory at this path, after removing all its contents. Use carefully!
///
/// # Errors
///
/// see `file::remove_file` and `fs::remove_dir`
#[doc(alias = "FSUSER_DeleteDirectoryRecursively")]
pub fn remove_dir_all<P: AsRef<Path>>(arch: &Archive, path: P) -> IoResult<()> {
    unsafe {
        let path = to_utf16(path.as_ref());
        let fs_path = ctru_sys::fsMakePath(PathType::UTF16.into(), path.as_ptr() as _);
        let r = ctru_sys::FSUSER_DeleteDirectoryRecursively(arch.handle, fs_path);
        if r < 0 {
            Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)))
        } else {
            Ok(())
        }
    }
}

/// Returns an iterator over the entries within a directory.
///
/// The iterator will yield instances of Result<DirEntry, i32>. New errors
/// may be encountered after an iterator is initially constructed.
///
/// This function will return an error in the following situations, but is not limited to just
/// these cases:
///
/// * The provided path doesn't exist.
/// * The process lacks permissions to view the contents.
/// * The path points at a non-directory file.
#[doc(alias = "FSUSER_OpenDirectory")]
pub fn read_dir<P: AsRef<Path>>(arch: &Archive, path: P) -> IoResult<ReadDir> {
    unsafe {
        let mut handle = 0;
        let root = Arc::new(path.as_ref().to_path_buf());
        let path = to_utf16(path.as_ref());
        let fs_path = ctru_sys::fsMakePath(PathType::UTF16.into(), path.as_ptr() as _);
        let r = ctru_sys::FSUSER_OpenDirectory(&mut handle, arch.handle, fs_path);
        if r < 0 {
            Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)))
        } else {
            Ok(ReadDir {
                handle: Dir(handle),
                root,
                arch,
            })
        }
    }
}

/// Removes a file from the filesystem.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not limited to just
/// these cases:
///
/// * path points to a directory.
/// * The user lacks permissions to remove the file.
#[doc(alias = "FSUSER_DeleteFile")]
pub fn remove_file<P: AsRef<Path>>(arch: &Archive, path: P) -> IoResult<()> {
    unsafe {
        let path = to_utf16(path.as_ref());
        let fs_path = ctru_sys::fsMakePath(PathType::UTF16.into(), path.as_ptr() as _);
        let r = ctru_sys::FSUSER_DeleteFile(arch.handle, fs_path);
        if r < 0 {
            Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)))
        } else {
            Ok(())
        }
    }
}

/// Rename a file or directory to a new name, replacing the original file
/// if to already exists.
///
/// # Errors
///
/// This function will return an error in the following situations, but is not limited to just
/// these cases:
///
/// * from does not exist.
/// * The user lacks permissions to view contents.
#[doc(alias = "FSUSER_RenameFile", alias = "FSUSER_RenameDirectory")]
pub fn rename<P, Q>(arch: &Archive, from: P, to: Q) -> IoResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    unsafe {
        let from = to_utf16(from.as_ref());
        let to = to_utf16(to.as_ref());

        let fs_from = ctru_sys::fsMakePath(PathType::UTF16.into(), from.as_ptr() as _);
        let fs_to = ctru_sys::fsMakePath(PathType::UTF16.into(), to.as_ptr() as _);

        let r = ctru_sys::FSUSER_RenameFile(arch.handle, fs_from, arch.handle, fs_to);
        if r == 0 {
            return Ok(());
        }
        let r = ctru_sys::FSUSER_RenameDirectory(arch.handle, fs_from, arch.handle, fs_to);
        if r == 0 {
            return Ok(());
        }
        Err(IoError::new(IoErrorKind::Other, ctru::Error::from(r)))
    }
}

// TODO: Determine if we should check UTF-16 paths for interior NULs
fn to_utf16(path: &Path) -> WideCString {
    WideCString::from_str(path).unwrap()
}

fn truncate_utf16_at_nul(v: &[u16]) -> &[u16] {
    match v.iter().position(|c| *c == 0) {
        // don't include the 0
        Some(i) => &v[..i],
        None => v,
    }
}

// Provides read_to_end functionality over an uninitialized buffer.
// This function is unsafe because it calls the underlying
// read function with a slice into uninitialized memory. The default
// implementation of read_to_end for readers will zero out new memory in
// the buf before passing it to read, but avoiding this zero can often
// lead to a fairly significant performance win.
//
// Implementations using this method have to adhere to two guarantees:
//  *  The implementation of read never reads the buffer provided.
//  *  The implementation of read correctly reports how many bytes were written.
unsafe fn read_to_end_uninitialized(r: &mut dyn Read, buf: &mut Vec<u8>) -> IoResult<usize> {
    let start_len = buf.len();
    buf.reserve(16);

    // Always try to read into the empty space of the vector (from the length to the capacity).
    // If the vector ever fills up then we reserve an extra byte which should trigger the normal
    // reallocation routines for the vector, which will likely double the size.
    //
    // This function is similar to the read_to_end function in std::io, but the logic about
    // reservations and slicing is different enough that this is duplicated here.
    loop {
        if buf.len() == buf.capacity() {
            buf.reserve(1);
        }

        let buf_slice =
            slice::from_raw_parts_mut(buf.as_mut_ptr().add(buf.len()), buf.capacity() - buf.len());

        match r.read(buf_slice) {
            Ok(0) => {
                return Ok(buf.len() - start_len);
            }
            Ok(n) => {
                let len = buf.len() + n;
                buf.set_len(len);
            }
            Err(ref e) if e.kind() == IoErrorKind::Interrupted => {}
            Err(e) => {
                return Err(e);
            }
        }
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> IoResult<usize> {
        self.read_to_end(buf)
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        match pos {
            SeekFrom::Start(off) => {
                self.offset = off;
            }
            SeekFrom::End(off) => {
                let mut temp = self.metadata()?.len() as i64;
                temp += off;
                self.offset = temp as u64;
            }
            SeekFrom::Current(off) => {
                let mut temp = self.offset as i64;
                temp += off;
                self.offset = temp as u64;
            }
        }
        Ok(self.offset)
    }
}

impl Drop for Archive {
    #[doc(alias = "FSUSER_CloseArchive")]
    fn drop(&mut self) {
        unsafe {
            let _ = ctru_sys::FSUSER_CloseArchive(self.handle);
        }
    }
}

impl Drop for File {
    #[doc(alias = "FSFILE_Close")]
    fn drop(&mut self) {
        unsafe {
            let _ = ctru_sys::FSFILE_Close(self.handle);
        }
    }
}

impl Drop for Dir {
    #[doc(alias = "FSDIR_Close")]
    fn drop(&mut self) {
        unsafe {
            let _ = ctru_sys::FSDIR_Close(self.0);
        }
    }
}
