# PhotoCraft source integration

Source: https://github.com/storytold/photocraft

Pinned commit: `ec350d64aedd019afc5eda290cc32909bc9bab7d` (the supplied reference checkout).

This directory is the complete native editor source, not the three-algorithm subset from CosKit beta.1. Original crate names, file format signatures, command IDs, tools, menu catalog and tests are retained. `upstream-manifest.json` records normalized SHA-256 values of the original implementation and assets. `integration-diff.patch` shows the reviewed integration changes; the additional `coskit_ai.rs` modules contain CosKit's native conversation integration. Run `python scripts/check-native-parity.py` from the CosKit root.

Product branding and build version are CosKit 1.0.0. Original copyright notices, contributor credits, MIT/Apache licenses, asset attribution and format acknowledgements remain. The non-open-source ArtCraft images under the upstream `docs/brand` directory are excluded. The app uses CosKit's existing icons, licensed under the repository's MIT license.

Inherited limitations remain documented and reported in CosKit's error messages. The original menu coverage is not a claim of complete Photoshop behavioral parity.

Beta.6 removes upstream promotional/contact entries from product Help and the welcome/title screens. About > Acknowledgements and the packaged ACKNOWLEDGEMENTS.md explicitly credit PhotoCraft, ArtCraft Team and upstream contributors. Copyright/license texts, contributor data, original hashes and compatibility identifiers remain intact.

CosKit v0.1.x and its conversational image editing preceded this integration. The native PhotoCraft implementation supplements the existing CosKit product with basic editing operations; its source provenance and original licenses remain unchanged.
