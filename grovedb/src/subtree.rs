//! Module for subtrees handling.
use ed::{Decode, Encode};
use merk::Op;

use crate::{Error, Merk};

/// Variants of GroveDB stored entities
#[derive(Debug, Decode, Encode, PartialEq)]
pub enum Element {
    /// An ordinary value
    Item(Vec<u8>),
    /// A reference to an object by its path
    Reference(Vec<u8>),
    /// A subtree, contains a root hash of the underlying Merk
    Tree([u8; 32]),
}

impl Element {
    // TODO: improve API to prevent creation of Tree elements with uncertain state
    pub fn empty_tree() -> Element {
        Element::Tree(Default::default())
    }

    pub fn new_reference(path: &[&[u8]], key: &[u8]) -> Self {
        Element::Reference(Self::build_merk_key(path.iter().map(|x| *x), key))
    }

    /// Recursively follow `Element::Reference`
    fn follow_reference(self, merk: &Merk) -> Result<Element, Error> {
        fn follow_reference_with_path(
            element: Element,
            merk: &Merk,
            paths: &mut Vec<Vec<u8>>,
        ) -> Result<Element, Error> {
            if let Element::Reference(reference_merk_key) = element {
                // Check if the reference merk key has been visited before
                // if it has then we have a cycle <return an error>
                if paths.contains(&reference_merk_key) {
                    return Err(Error::CyclicReferencePath);
                }
                let element = Element::decode(
                    merk.get(reference_merk_key.as_slice())?
                        .ok_or(Error::InvalidPath("key not found in Merk"))?
                        .as_slice(),
                )?;

                paths.push(reference_merk_key);
                follow_reference_with_path(element, merk, paths)
            } else {
                Ok(element)
            }
        }

        let mut reference_paths: Vec<Vec<u8>> = Vec::new();
        follow_reference_with_path(self, merk, &mut reference_paths)
    }

    /// A helper method to build Merk keys (and RocksDB as well) out of path +
    /// key
    fn build_merk_key<'a>(path: impl Iterator<Item = &'a [u8]>, key: &'a [u8]) -> Vec<u8> {
        let mut merk_key = path.fold(Vec::<u8>::new(), |mut acc, p| {
            acc.extend(p.into_iter());
            acc
        });
        merk_key.extend(key);
        merk_key
    }

    /// Get an element from Merk under a key; path should be resolved and proper
    /// Merk should be loaded by this moment
    pub fn get(merk: &Merk, key: &[u8]) -> Result<Element, Error> {
        let element = Element::decode(
            merk.get(&key)?
                .ok_or(Error::InvalidPath("key not found in Merk"))?
                .as_slice(),
        )?;
        // TODO: fix `follow_reference` as now it is possible to jump between multiple
        // merks element.follow_reference(&merk)
        todo!()
    }

    /// Insert an element in Merk under a key; path should be resolved and
    /// proper Merk should be loaded by this moment
    pub fn insert(&self, merk: &mut Merk, key: Vec<u8>) -> Result<(), Error> {
        let batch = [(key, Op::Put(Element::encode(self)?))];
        merk.apply(&batch, &[]).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;

    #[test]
    fn test_success_insert() {
        let tmp_dir = TempDir::new("db").unwrap();
        let mut merk = Merk::open(tmp_dir.path()).unwrap();
        Element::Tree
            .insert(&mut merk, &[], b"mykey")
            .expect("expected successful insertion");
        Element::Item(b"value".to_vec())
            .insert(&mut merk, &[b"mykey"], b"another-key")
            .expect("expected successful insertion 2");

        assert_eq!(
            Element::get(&merk, &[b"mykey"], b"another-key").expect("expected successful get"),
            Element::Item(b"value".to_vec()),
        );
    }

    #[test]
    fn test_follow_references() {
        let tmp_dir = TempDir::new("db").unwrap();
        let mut merk = Merk::open(tmp_dir.path()).unwrap();
        Element::Tree
            .insert(&mut merk, &[], b"mykey")
            .expect("expected successful insertion");
        Element::Item(b"value".to_vec())
            .insert(&mut merk, &[b"mykey"], b"another-key")
            .expect("expected successful insertion 2");
        Element::new_reference(&[b"mykey"], b"another-key")
            .insert(&mut merk, &[b"mykey"], b"reference")
            .expect("expected successful reference insertion");
        Element::new_reference(&[b"mykey"], b"reference")
            .insert(&mut merk, &[b"mykey"], b"another-reference")
            .expect("expected successful reference insertion 2");

        assert_eq!(
            Element::get(&merk, &[b"mykey"], b"another-reference")
                .expect("expected successful get"),
            Element::Item(b"value".to_vec()),
        );
    }

    #[test]
    fn test_circular_references() {
        let tmp_dir = TempDir::new("db").unwrap();
        let mut merk = Merk::open(tmp_dir.path()).unwrap();

        Element::Tree
            .insert(&mut merk, &[], b"tree-key")
            .expect("expected successful insertion");

        // r1 points to r2 and r2 points to r1 (cycle!)
        Element::new_reference(&[b"tree-key"], b"reference-2")
            .insert(&mut merk, &[b"tree-key"], b"reference-1")
            .expect("expected successful reference insertion");
        Element::new_reference(&[b"tree-key"], b"reference-1")
            .insert(&mut merk, &[b"tree-key"], b"reference-2")
            .expect("expected successful reference insertion");

        assert!(Element::get(&merk, &[b"tree-key"], b"reference-1").is_err());
    }
}
