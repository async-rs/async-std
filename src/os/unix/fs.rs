//! Unix-specific filesystem extensions.

use crate::io;
use crate::path::Path;
use crate::task::spawn_blocking;

/// Creates a new symbolic link on the filesystem.
///
/// The `dst` path will be a symbolic link pointing to the `src` path.
///
/// This function is an async version of [`std::os::unix::fs::symlink`].
///
/// [`std::os::unix::fs::symlink`]: https://doc.rust-lang.org/std/os/unix/fs/fn.symlink.html
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::os::unix::fs::symlink;
///
/// symlink("a.txt", "b.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<()> {
    let src = src.as_ref().to_owned();
    let dst = dst.as_ref().to_owned();
    spawn_blocking(move || std::os::unix::fs::symlink(&src, &dst)).await
}

cfg_not_docs! {
    pub use std::os::unix::fs::{DirBuilderExt, DirEntryExt, OpenOptionsExt, FileExt};
}

cfg_docs! {
    use async_trait::async_trait;

    /// Unix-specific extensions to `DirBuilder`.
    pub trait DirBuilderExt {
        /// Sets the mode to create new directories with. This option defaults to
        /// `0o777`.
        fn mode(&mut self, mode: u32) -> &mut Self;
    }

    /// Unix-specific extension methods for `DirEntry`.
    pub trait DirEntryExt {
        /// Returns the underlying `d_ino` field in the contained `dirent`
        /// structure.
        fn ino(&self) -> u64;
    }

    /// Unix-specific extensions to `OpenOptions`.
    pub trait OpenOptionsExt {
        /// Sets the mode bits that a new file will be created with.
        ///
        /// If a new file is created as part of a `File::open_opts` call then this
        /// specified `mode` will be used as the permission bits for the new file.
        /// If no `mode` is set, the default of `0o666` will be used.
        /// The operating system masks out bits with the systems `umask`, to produce
        /// the final permissions.
        fn mode(&mut self, mode: u32) -> &mut Self;

        /// Pass custom flags to the `flags` argument of `open`.
        ///
        /// The bits that define the access mode are masked out with `O_ACCMODE`, to
        /// ensure they do not interfere with the access mode set by Rusts options.
        ///
        /// Custom flags can only set flags, not remove flags set by Rusts options.
        /// This options overwrites any previously set custom flags.
        fn custom_flags(&mut self, flags: i32) -> &mut Self;
    }

    /// Unix-specific extensions to [`fs::File`].
    #[async_trait]
    pub trait FileExt {
        /// Reads a number of bytes starting from a given offset.
        ///
        /// Returns the number of bytes read.
        ///
        /// The offset is relative to the start of the file and thus independent
        /// from the current cursor.
        ///
        /// The current file cursor is not affected by this function.
        ///
        /// Note that similar to [`File::read`], it is not an error to return with a
        /// short read.
        ///
        /// [`File::read`]: fs::File::read
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use async_std::io;
        /// use async_std::fs::File;
        /// use async_std::os::unix::prelude::FileExt;
        ///
        /// async fn main() -> io::Result<()> {
        ///     let mut buf = [0u8; 8];
        ///     let file = File::open("foo.txt").await?;
        ///
        ///     // We now read 8 bytes from the offset 10.
        ///     let num_bytes_read = file.read_at(&mut buf, 10).await?;
        ///     println!("read {} bytes: {:?}", num_bytes_read, buf);
        ///     Ok(())
        /// }
        /// ```
        async fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize>;

        /// Reads the exact number of byte required to fill `buf` from the given offset.
        ///
        /// The offset is relative to the start of the file and thus independent
        /// from the current cursor.
        ///
        /// The current file cursor is not affected by this function.
        ///
        /// Similar to [`io::Read::read_exact`] but uses [`read_at`] instead of `read`.
        ///
        /// [`read_at`]: FileExt::read_at
        ///
        /// # Errors
        ///
        /// If this function encounters an error of the kind
        /// [`io::ErrorKind::Interrupted`] then the error is ignored and the operation
        /// will continue.
        ///
        /// If this function encounters an "end of file" before completely filling
        /// the buffer, it returns an error of the kind [`io::ErrorKind::UnexpectedEof`].
        /// The contents of `buf` are unspecified in this case.
        ///
        /// If any other read error is encountered then this function immediately
        /// returns. The contents of `buf` are unspecified in this case.
        ///
        /// If this function returns an error, it is unspecified how many bytes it
        /// has read, but it will never read more than would be necessary to
        /// completely fill the buffer.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use async_std::io;
        /// use async_std::fs::File;
        /// use async_std::os::unix::prelude::FileExt;
        ///
        /// async fn main() -> io::Result<()> {
        ///     let mut buf = [0u8; 8];
        ///     let file = File::open("foo.txt").await?;
        ///
        ///     // We now read exactly 8 bytes from the offset 10.
        ///     file.read_exact_at(&mut buf, 10).await?;
        ///     println!("read {} bytes: {:?}", buf.len(), buf);
        ///     Ok(())
        /// }
        /// ```
        async fn read_exact_at(&self, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
            while !buf.is_empty() {
                match self.read_at(buf, offset).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let tmp = buf;
                        buf = &mut tmp[n..];
                        offset += n as u64;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(e) => return Err(e),
                }
            }
            if !buf.is_empty() {
                Err(io::Error::new(io::ErrorKind::UnexpectedEof, "failed to fill whole buffer"))
            } else {
                Ok(())
            }
        }

        /// Writes a number of bytes starting from a given offset.
        ///
        /// Returns the number of bytes written.
        ///
        /// The offset is relative to the start of the file and thus independent
        /// from the current cursor.
        ///
        /// The current file cursor is not affected by this function.
        ///
        /// When writing beyond the end of the file, the file is appropriately
        /// extended and the intermediate bytes are initialized with the value 0.
        ///
        /// Note that similar to [`File::write`], it is not an error to return a
        /// short write.
        ///
        /// [`File::write`]: fs::File::write
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use async_std::fs::File;
        /// use async_std::io;
        /// use async_std::os::unix::prelude::FileExt;
        ///
        /// async fn main() -> io::Result<()> {
        ///     let file = File::open("foo.txt").await?;
        ///
        ///     // We now write at the offset 10.
        ///     file.write_at(b"sushi", 10).await?;
        ///     Ok(())
        /// }
        /// ```
        async fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize>;

        /// Attempts to write an entire buffer starting from a given offset.
        ///
        /// The offset is relative to the start of the file and thus independent
        /// from the current cursor.
        ///
        /// The current file cursor is not affected by this function.
        ///
        /// This method will continuously call [`write_at`] until there is no more data
        /// to be written or an error of non-[`io::ErrorKind::Interrupted`] kind is
        /// returned. This method will not return until the entire buffer has been
        /// successfully written or such an error occurs. The first error that is
        /// not of [`io::ErrorKind::Interrupted`] kind generated from this method will be
        /// returned.
        ///
        /// # Errors
        ///
        /// This function will return the first error of
        /// non-[`io::ErrorKind::Interrupted`] kind that [`write_at`] returns.
        ///
        /// [`write_at`]: FileExt::write_at
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use async_std::fs::File;
        /// use async_std::io;
        /// use async_std::os::unix::prelude::FileExt;
        ///
        /// async fn main() -> io::Result<()> {
        ///     let file = File::open("foo.txt").await?;
        ///
        ///     // We now write at the offset 10.
        ///     file.write_all_at(b"sushi", 10).await?;
        ///     Ok(())
        /// }
        /// ```
        async fn write_all_at(&self, mut buf: &[u8], mut offset: u64) -> io::Result<()> {
            while !buf.is_empty() {
                match self.write_at(buf, offset).await {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "failed to write whole buffer",
                        ));
                    }
                    Ok(n) => {
                        buf = &buf[n..];
                        offset += n as u64
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(e) => return Err(e),
                }
            }
            Ok(())
        }
    }
}
