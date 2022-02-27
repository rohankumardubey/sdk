// Copyright 2019-2022 Manta Network.
// This file is part of manta-sdk.
//
// manta-sdk is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// manta-sdk is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with manta-sdk.  If not, see <http://www.gnu.org/licenses/>.

//! Manta SDK

// TODO: Check checksums when decoding or maybe also when downloading.
// TODO: Use more code-generation to reduce duplication here.
// TODO: Have a method of downloading all data from each category.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![forbid(rustdoc::broken_intra_doc_links)]
#![forbid(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "download")]
use {anyhow::Result, std::path::Path};

/// GitHub Data File Downloading
#[cfg(feature = "download")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "download")))]
pub mod github {
    use super::*;
    use std::fs::{File, OpenOptions};

    /// GitHub Organization
    pub const ORGANIZATION: &str = "manta-network";

    /// SDK GitHub Repository Name
    pub const REPO: &str = "sdk";

    /// Default GitHub Branch
    pub const DEFAULT_BRANCH: &str = "main";

    /// Returns the Git-LFS URL for GitHub content at the given `branch` and `data_path`.
    #[inline]
    pub fn lfs_url(branch: &str, data_path: &str) -> String {
        alloc::format!(
            "https://media.githubusercontent.com/media/{ORGANIZATION}/{REPO}/{branch}/{data_path}"
        )
    }

    /// Returns the raw file storage URL for GitHub content at the given `branch` and `data_path`.
    #[inline]
    pub fn raw_url(branch: &str, data_path: &str) -> String {
        alloc::format!(
            "https://raw.githubusercontent.com/{ORGANIZATION}/{REPO}/{branch}/{data_path}"
        )
    }

    /// Downloads the data from `url` to `file` returning the number of bytes read.
    #[inline]
    fn download_from(url: String, file: &mut File) -> Result<u64> {
        Ok(attohttpc::get(url).send()?.write_to(file)?)
    }

    /// Downloads data from `data_path` relative to the given `branch` to a file at `path` without
    /// checking any checksums.
    ///
    /// # Safety
    ///
    /// Prefer the [`download`] method which checks the data against a given checksum.
    #[inline]
    pub fn download_unchecked<P>(branch: &str, data_path: &str, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let mut file = OpenOptions::new().create(true).write(true).open(path)?;
        if download_from(lfs_url(branch, data_path), &mut file)? == 0 {
            download_from(raw_url(branch, data_path), &mut file)?;
        }
        Ok(())
    }

    /// Downloads data from `data_path` relative to the given `branch` to a file at `path` verifying
    /// that the data matches the `checksum`.
    #[inline]
    pub fn download<P>(branch: &str, data_path: &str, path: P, checksum: &[u8; 32]) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        download_unchecked(branch, data_path, &path)?;
        anyhow::ensure!(verify_file(path, checksum)?, "Checksum did not match.");
        Ok(())
    }
}

/// Verifies the `data` against the `checksum`.
#[inline]
pub fn verify(data: &[u8], checksum: &[u8; 32]) -> bool {
    &blake3::hash(data) == checksum
}

/// Verifies the data in the file located at `path` against the `checksum`.
#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
#[inline]
pub fn verify_file<P>(path: P, checksum: &[u8; 32]) -> std::io::Result<bool>
where
    P: AsRef<Path>,
{
    Ok(verify(&std::fs::read(path)?, checksum))
}

/// Defines a data marker type loading its raw data and checksum from disk.
macro_rules! define {
    ($name:tt, $doc:expr, $path:expr $(,)?) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name;

        impl $name {
            #[doc = $doc]
            #[doc = "Data Bytes"]
            pub const DATA: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), $path, ".dat"));

            #[doc = $doc]
            #[doc = "Data Checksum"]
            pub const CHECKSUM: &'static [u8; 32] =
                include_bytes!(concat!(env!("OUT_DIR"), $path, ".checksum"));

            /// Verifies that [`Self::DATA`] is consistent against [`Self::CHECKSUM`].
            #[inline]
            pub fn verify() -> bool {
                crate::verify(Self::DATA, Self::CHECKSUM)
            }

            /// Gets the underlying binary data after verifying against [`Self::CHECKSUM`].
            #[inline]
            pub fn get() -> Option<&'static [u8]> {
                if Self::verify() {
                    Some(Self::DATA)
                } else {
                    None
                }
            }
        }
    };
}

/// Defines the data marker type for download-required data and checksum from disk.
macro_rules! define_download {
    ($name:tt, $doc:expr, $path:expr $(,)?) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name;

        impl $name {
            #[doc = $doc]
            #[doc = "Data Checksum"]
            pub const CHECKSUM: &'static [u8; 32] =
                include_bytes!(concat!(env!("OUT_DIR"), $path, ".checksum"));

            #[doc = "Downloads the data for the"]
            #[doc = $doc]
            #[doc = r"from GitHub. This method automatically verifies the checksum when downloading.
                      See [`github::download`](crate::github::download) for more."]
            #[cfg(feature = "download")]
            #[cfg_attr(doc_cfg, doc(cfg(feature = "download")))]
            #[inline]
            pub fn download<P>(path: P) -> Result<()>
            where
                P: AsRef<Path>,
            {
                github::download(
                    github::DEFAULT_BRANCH,
                    concat!($path, ".dat"),
                    path,
                    Self::CHECKSUM,
                )
            }

            #[doc = "Checks if the data for the"]
            #[doc = $doc]
            #[doc = r"matches the checksum and if not downloads it from GitHub. This method
                      automatically verifies the checksum when downloading.
                      See [`github::download`](crate::github::download) for more."]
            #[cfg(feature = "download")]
            #[cfg_attr(doc_cfg, doc(cfg(feature = "download")))]
            #[inline]
            pub fn download_if_invalid<P>(path: P) -> Result<()>
            where
                P: AsRef<Path>,
            {
                match verify_file(&path, Self::CHECKSUM) {
                    Ok(true) => Ok(()),
                    _ => Self::download(path),
                }
            }
        }
    };
}

/// Manta Pay
///
/// See [`manta-pay`](https://github.com/manta-network/manta-rs) for the definitions.
pub mod pay {
    #[cfg(feature = "download")]
    use super::*;

    /// Testnet Data
    pub mod testnet {
        #[cfg(feature = "download")]
        use super::*;

        /// Parameters
        pub mod parameters {
            define!(
                KeyAgreement,
                "Key Agreement Scheme Parameters",
                "/data/pay/testnet/parameters/key-agreement",
            );
            define!(
                UtxoCommitmentScheme,
                "UTXO Commitment Scheme Parameters",
                "/data/pay/testnet/parameters/utxo-commitment-scheme",
            );
            define!(
                VoidNumberHashFunction,
                "Void Number Hash Function Parameters",
                "/data/pay/testnet/parameters/void-number-hash-function",
            );
            define!(
                UtxoSetParameters,
                "UTXO Set Parameters",
                "/data/pay/testnet/parameters/utxo-set-parameters",
            );
        }

        /// Zero-Knowledge Proof System Proving Data
        pub mod proving {
            #[cfg(feature = "download")]
            use super::*;

            define_download!(
                Mint,
                "Mint Proving Context",
                "/data/pay/testnet/proving/mint",
            );
            define_download!(
                PrivateTransfer,
                "Private Transfer Proving Context",
                "/data/pay/testnet/proving/private-transfer",
            );
            define_download!(
                Reclaim,
                "Reclaim Proving Context",
                "/data/pay/testnet/proving/reclaim",
            );
        }

        /// Zero-Knowledge Proof System Verifying Data
        pub mod verifying {
            define!(
                Mint,
                "Mint Verifying Context",
                "/data/pay/testnet/verifying/mint"
            );
            define!(
                PrivateTransfer,
                "Private Transfer Verifying Context",
                "/data/pay/testnet/verifying/private-transfer"
            );
            define!(
                Reclaim,
                "Reclaim Verifying Context",
                "/data/pay/testnet/verifying/reclaim"
            );
        }
    }
}

/// Testing Suite
#[cfg(test)]
mod test {
    use super::*;
    use anyhow::{anyhow, bail, Result};
    use hex::FromHex;
    use std::{
        collections::HashMap,
        fs::{self, File, OpenOptions},
        io::{BufRead, BufReader, Read},
        path::PathBuf,
    };

    /// Checks if two files `lhs` and `rhs` have equal content.
    #[inline]
    fn equal_files(lhs: &mut File, rhs: &mut File) -> Result<bool> {
        let mut lhs_buffer = [0; 2048];
        let mut rhs_buffer = [0; 2048];
        loop {
            let lhs_len = lhs.read(&mut lhs_buffer)?;
            let rhs_len = rhs.read(&mut rhs_buffer)?;
            if (lhs_len != rhs_len) || (lhs_buffer[..lhs_len] != rhs_buffer[..rhs_len]) {
                return Ok(false);
            }
            if lhs_len == 0 {
                return Ok(true);
            }
        }
    }

    /// Checksum
    type Checksum = [u8; 32];

    /// Checksum Map
    type ChecksumMap = HashMap<PathBuf, Checksum>;

    /// Parses the checkfile at `path` producing a [`ChecksumMap`] for all the files in the data
    /// directory.
    #[inline]
    fn parse_checkfile<P>(path: P) -> Result<ChecksumMap>
    where
        P: AsRef<Path>,
    {
        let file = OpenOptions::new().read(true).open(path)?;
        let mut checksums = ChecksumMap::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let mut iter = line.split("  ");
            match (iter.next(), iter.next(), iter.next()) {
                (Some(checksum), Some(path), None) => {
                    checksums.insert(path.into(), Checksum::from_hex(checksum)?);
                }
                _ => bail!("Invalid checkfile line: {:?}", line),
            }
        }
        Ok(checksums)
    }

    /// Gets the checksum from the `checksums` map for `path` returning an error if it was not found.
    #[inline]
    fn get_checksum<P>(checksums: &ChecksumMap, path: P) -> Result<Checksum>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        checksums
            .get(path)
            .ok_or_else(|| anyhow!("Unable to get checksum for path: {:?}", path))
            .map(move |c| *c)
    }

    /// Downloads all data from GitHub and checks if they are the same as the data known locally to
    /// this Rust crate.
    #[test]
    fn download_all_data() -> Result<()> {
        let directory = tempfile::tempdir().expect("Unable to generate temporary test directory.");
        println!("[INFO] Temporary Directory: {:?}", directory);
        let checksums = parse_checkfile("data.checkfile")?;
        let directory_path = directory.path();
        for file in walkdir::WalkDir::new("data") {
            let file = file?;
            let path = file.path();
            if !path.is_dir() {
                println!("[INFO] Checking path: {:?}", path);
                let target = directory_path.join(path);
                fs::create_dir_all(target.parent().unwrap())?;
                github::download(
                    github::DEFAULT_BRANCH,
                    path.to_str().unwrap(),
                    &target,
                    &get_checksum(&checksums, path)?,
                )?;
                assert!(equal_files(
                    &mut File::open(path)?,
                    &mut File::open(target)?
                )?);
            }
        }
        Ok(())
    }
}
