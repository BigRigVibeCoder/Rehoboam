// Positive control (VER-SPR-008): renaming the include macro would hide an include from the gate,
// so any `use` that names it FAILS. Never compiled.
use core::include as inc;
inc!("/etc/hostname");
