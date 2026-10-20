//! The RNG purposes of world generation (SPR-011 ruling 10). Every draw is keyed by
//! `H(world_seed, branch_salt = 0, system_id("rh-worldgen"), epoch 0, purpose, entity, t = 0, k)`:
//! the reserved id `rh-worldgen` (`rh_core::register::RESERVED_SYSTEM_IDS`), the founder's
//! `PersonId` as the entity, genesis (tick 0) as the instant. Key: (founder, genesis).
//!
//! SPR-018 adds the heightmap purposes for worldgen-lite: `heightmap_blob` for W1 blob draws and
//! `region_seed` for W6's first seed.

/// A purpose world generation draws under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    /// k = 0: a founder's sex, when the manifest gives none (a fair draw).
    FounderSex,
    /// k = 0: a founder's age in whole years, when the manifest gives none (uniform in the role's
    /// range).
    FounderAge,
    /// k = 0: the founder's birthday, an offset in minutes inside the year of birth.
    FounderBirthday,
    /// W1: keyed blob parameters (k = 0..3 for domes, k = 0..4 for rings). Entity = blob ordinal.
    HeightmapBlob,
    /// W6: the first region seed. Entity = 0, k = 0.
    RegionSeed,
}

impl RngPurpose {
    /// Every purpose.
    pub const ALL: [Self; 5] = [
        Self::FounderSex,
        Self::FounderAge,
        Self::FounderBirthday,
        Self::HeightmapBlob,
        Self::RegionSeed,
    ];

    /// The purpose's name (hashed into the stream key by `rh_ids::PurposeId::of`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FounderSex => "founder_sex",
            Self::FounderAge => "founder_age",
            Self::FounderBirthday => "founder_birthday",
            Self::HeightmapBlob => "heightmap_blob",
            Self::RegionSeed => "region_seed",
        }
    }
}
