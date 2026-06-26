pub use keys::CpcKey;
use std::collections::HashSet;

mod keys;

pub struct Keyboard {
    pressed_keys: HashSet<CpcKey>,
}

impl Keyboard {
    pub fn new() -> Self {
        Keyboard {
            pressed_keys: HashSet::with_capacity(CpcKey::ALL_KEYS.len()),
        }
    }

    pub fn is_pressed(&self, key: &CpcKey) -> bool {
        self.pressed_keys.contains(key)
    }

    pub fn press_key(&mut self, key: &CpcKey) {
        self.pressed_keys.insert(*key);
    }

    pub fn release_key(&mut self, key: &CpcKey) {
        self.pressed_keys.remove(key);
    }

    pub fn any_key_pressed(&self) -> bool {
        !self.pressed_keys.is_empty()
    }

    pub fn pressed_keys_count(&self) -> usize {
        self.pressed_keys.len()
    }

    pub fn reset(&mut self) {
        self.pressed_keys.clear();
    }

    fn select_row(row: u8) -> &'static [CpcKey] {
        match row {
            0 => &CpcKey::ROW1,
            1 => &CpcKey::ROW2,
            2 => &CpcKey::ROW3,
            3 => &CpcKey::ROW4,
            4 => &CpcKey::ROW5,
            5 => &CpcKey::ROW6,
            6 => &CpcKey::ROW7,
            7 => &CpcKey::ROW8,
            8 => &CpcKey::ROW9,
            9 => &CpcKey::ROW10,
            _ => panic!("Unexpected row {row}"),
        }
    }

    fn ghosted(&self, row: u8, idx: usize) -> bool {
        for neighbour_idx in 0..8 {
            if neighbour_idx == idx {
                continue;
            }

            for neighbour_row in 0..8 {
                if neighbour_row == row {
                    continue;
                }

                if self.is_pressed(&Self::select_row(neighbour_row)[idx])
                    && self.is_pressed(&Self::select_row(neighbour_row)[neighbour_idx])
                    && self.is_pressed(&Self::select_row(row)[neighbour_idx])
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn read_row(&self, row_idx: u8) -> u8 {
        let mut result = 0xFF;
        let row = Self::select_row(row_idx);

        for (idx, key) in row.iter().enumerate() {
            if self.is_pressed(key) || self.ghosted(row_idx, idx) {
                result &= !(1 << (7 - idx));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Construction and Initialization Tests
    // =========================================================================

    #[test]
    fn new_keyboard_has_all_keys_unpressed() {
        let keyboard = Keyboard::new();

        for key in CpcKey::ALL_KEYS {
            assert!(
                !keyboard.is_pressed(&key),
                "Key {:?} should not be pressed on new keyboard",
                key
            );
        }
    }

    #[test]
    fn new_keyboard_all_rows_return_all_bits_set() {
        let keyboard = Keyboard::new();

        // On CPC, unpressed keys return all 1s
        for row in 0..8 {
            let row_value = keyboard.read_row(row);
            assert_eq!(
                row_value, 0xFF,
                "Row {} should return 0xFF when no keys pressed",
                row
            );
        }
    }

    // =========================================================================
    // Single Key Press/Release Tests
    // =========================================================================

    #[test]
    fn press_key_reports_as_pressed() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::A);
        assert!(keyboard.is_pressed(&CpcKey::A));
    }

    #[test]
    fn press_key_only_affects_that_key() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::A);

        assert!(keyboard.is_pressed(&CpcKey::A));
        assert!(!keyboard.is_pressed(&CpcKey::S));
        assert!(!keyboard.is_pressed(&CpcKey::D));
        assert!(!keyboard.is_pressed(&CpcKey::Q));
    }

    #[test]
    fn release_key_reports_as_unpressed() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::A);
        keyboard.release_key(&CpcKey::A);

        assert!(!keyboard.is_pressed(&CpcKey::A));
    }

    #[test]
    fn release_unpressed_key_does_nothing() {
        let mut keyboard = Keyboard::new();

        // Should not panic or cause issues
        keyboard.release_key(&CpcKey::A);
        assert!(!keyboard.is_pressed(&CpcKey::A));
    }

    #[test]
    fn press_already_pressed_key_stays_pressed() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::A);
        keyboard.press_key(&CpcKey::A);

        assert!(keyboard.is_pressed(&CpcKey::A));
    }

    #[test]
    fn press_then_release_different_keys() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::A);
        keyboard.press_key(&CpcKey::S);

        keyboard.release_key(&CpcKey::A);

        assert!(!keyboard.is_pressed(&CpcKey::A));
        assert!(keyboard.is_pressed(&CpcKey::S));
    }

    // =========================================================================
    // Multiple Key Press Tests
    // =========================================================================

    #[test]
    fn two_keys_in_same_row() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::Three); // bit 1
        keyboard.press_key(&CpcKey::Four); // bit 0

        assert_eq!(keyboard.read_row(7), 0xFC); // bits 0 and 1 cleared
    }

    #[test]
    fn three_keys_in_same_row() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::D);
        keyboard.press_key(&CpcKey::W);
        keyboard.press_key(&CpcKey::Four);

        assert_eq!(keyboard.read_row(7), 0b11010110);
    }

    #[test]
    fn all_keys_in_row_pressed() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::X);
        keyboard.press_key(&CpcKey::C);
        keyboard.press_key(&CpcKey::D);
        keyboard.press_key(&CpcKey::S);
        keyboard.press_key(&CpcKey::W);
        keyboard.press_key(&CpcKey::E);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::Four);

        assert_eq!(keyboard.read_row(7), 0);
    }

    #[test]
    fn keys_in_different_rows_dont_interfere() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::Four); // Row 8, bit 0
        keyboard.press_key(&CpcKey::Six); // Row 7, bit 0

        assert_eq!(keyboard.read_row(7), 0xFE);
        assert_eq!(keyboard.read_row(6), 0xFE);
        assert_eq!(keyboard.read_row(0), 0xFF);
    }

    #[test]
    fn release_one_of_multiple_pressed_keys() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);

        keyboard.release_key(&CpcKey::Three);

        assert_eq!(keyboard.read_row(7), 0xFE);
    }

    #[test]
    fn cursor_keys_simulation() {
        // Cursor keys on Spectrum are achieved with Caps Shift + 5,6,7,8
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::Shift); // Row 1
        keyboard.press_key(&CpcKey::F8); // Row 5, bit 2 (cursor up)

        assert!(keyboard.is_pressed(&CpcKey::Shift));
        assert!(keyboard.is_pressed(&CpcKey::F8));
    }

    // =========================================================================
    // Reset Tests
    // =========================================================================

    #[test]
    fn reset_clears_all_pressed_keys() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::A);
        keyboard.press_key(&CpcKey::Q);
        keyboard.press_key(&CpcKey::Space);
        keyboard.press_key(&CpcKey::Enter);

        keyboard.reset();

        assert!(!keyboard.is_pressed(&CpcKey::A));
        assert!(!keyboard.is_pressed(&CpcKey::Q));
        assert!(!keyboard.is_pressed(&CpcKey::Space));
        assert!(!keyboard.is_pressed(&CpcKey::Enter));
    }

    #[test]
    fn reset_returns_all_rows_to_unpressed() {
        let mut keyboard = Keyboard::new();

        // Press at least one key in each row
        keyboard.press_key(&CpcKey::Shift); // Row 1
        keyboard.press_key(&CpcKey::A); // Row 2
        keyboard.press_key(&CpcKey::Q); // Row 3
        keyboard.press_key(&CpcKey::F1); // Row 4
        keyboard.press_key(&CpcKey::F0); // Row 5
        keyboard.press_key(&CpcKey::P); // Row 6
        keyboard.press_key(&CpcKey::Enter); // Row 7
        keyboard.press_key(&CpcKey::Space); // Row 8

        keyboard.reset();

        for row in 0..8 {
            assert_eq!(keyboard.read_row(row), 0xFF);
        }
    }

    #[test]
    fn reset_on_empty_keyboard_does_nothing() {
        let mut keyboard = Keyboard::new();

        keyboard.reset();

        for row in 0..8 {
            assert_eq!(keyboard.read_row(row), 0xFF);
        }
    }

    // =========================================================================
    // Boundary and Edge Case Tests
    // =========================================================================

    #[test]
    fn rapid_press_release_cycles() {
        let mut keyboard = Keyboard::new();

        for _ in 0..1000 {
            keyboard.press_key(&CpcKey::A);
            assert!(keyboard.is_pressed(&CpcKey::A));

            keyboard.release_key(&CpcKey::A);
            assert!(!keyboard.is_pressed(&CpcKey::A));
        }
    }

    #[test]
    fn all_keys_pressed_simultaneously() {
        let mut keyboard = Keyboard::new();

        for key in CpcKey::ALL_KEYS {
            keyboard.press_key(&key);
        }

        for key in CpcKey::ALL_KEYS {
            assert!(keyboard.is_pressed(&key), "Key {:?} should be pressed", key);
        }

        // All rows should return 0
        for row in 0..10 {
            assert_eq!(keyboard.read_row(row), 0x00);
        }
    }

    #[test]
    fn release_all_keys_after_all_pressed() {
        let mut keyboard = Keyboard::new();

        for key in CpcKey::ALL_KEYS {
            keyboard.press_key(&key);
        }

        for key in CpcKey::ALL_KEYS {
            keyboard.release_key(&key);
        }

        for row in 0..8 {
            assert_eq!(keyboard.read_row(row), 0xFF);
        }
    }

    #[test]
    fn pressing_same_key_multiple_times_is_idempotent() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Four);

        assert_eq!(keyboard.read_row(7), 0xFE); // Only bit 0 cleared
    }

    #[test]
    fn releasing_same_key_multiple_times_is_idempotent() {
        let mut keyboard = Keyboard::new();

        keyboard.press_key(&CpcKey::A);
        keyboard.release_key(&CpcKey::A);
        keyboard.release_key(&CpcKey::A);
        keyboard.release_key(&CpcKey::A);

        assert_eq!(keyboard.read_row(1), 0xFF);
    }

    // =========================================================================
    // State Query Tests
    // =========================================================================

    #[test]
    fn get_state_of_pressed_key() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::A);

        assert_eq!(keyboard.is_pressed(&CpcKey::A), true);
    }

    #[test]
    fn get_state_of_released_key() {
        let keyboard = Keyboard::new();

        assert_eq!(keyboard.is_pressed(&CpcKey::A), false);
    }

    #[test]
    fn any_key_pressed_returns_false_when_none_pressed() {
        let keyboard = Keyboard::new();
        assert!(!keyboard.any_key_pressed());
    }

    #[test]
    fn any_key_pressed_returns_true_when_any_pressed() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Z);
        assert!(keyboard.any_key_pressed());
    }

    #[test]
    fn get_pressed_keys_count_zero() {
        let keyboard = Keyboard::new();
        assert_eq!(keyboard.pressed_keys_count(), 0);
    }

    #[test]
    fn get_pressed_keys_count_multiple() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::A);
        keyboard.press_key(&CpcKey::S);
        keyboard.press_key(&CpcKey::D);
        assert_eq!(keyboard.pressed_keys_count(), 3);
    }

    #[test]
    fn get_pressed_keys_count_after_release() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::A);
        keyboard.press_key(&CpcKey::S);
        keyboard.release_key(&CpcKey::A);
        assert_eq!(keyboard.pressed_keys_count(), 1);
    }

    #[test]
    fn ghosting_three_keys_forming_rectangle_creates_ghost() {
        let mut keyboard = Keyboard::new();
        // Rectangle corners:
        //   Four  (row 7, bit 0)
        //   Three (row 7, bit 1)
        //   Six   (row 6, bit 0)
        // Ghost appears at (row 6, bit 1)
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::Six);

        // Row 7: Four + Three → bits 0, 1 cleared
        assert_eq!(keyboard.read_row(7), 0b11111100);
        // Row 6: Six (bit 0) + ghost (bit 1) → bits 0, 1 cleared
        assert_eq!(keyboard.read_row(6), 0b11111100);
    }

    #[test]
    fn ghosting_disappears_when_shared_corner_released() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::Six);

        // Ghost present
        assert_eq!(keyboard.read_row(6), 0b11111100);

        // Release Six — rectangle broken
        keyboard.release_key(&CpcKey::Six);

        // No ghost in row 6
        assert_eq!(keyboard.read_row(6), 0xFF);
        // Four and Three still pressed in row 7
        assert_eq!(keyboard.read_row(7), 0b11111100);
    }

    #[test]
    fn ghosting_disappears_when_same_row_key_released() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::Six);

        assert_eq!(keyboard.read_row(6), 0b11111100);

        // Release Three — rectangle broken, Four and Six share column but no third corner
        keyboard.release_key(&CpcKey::Three);

        assert_eq!(keyboard.read_row(6), 0b11111110); // Only Six
        assert_eq!(keyboard.read_row(7), 0b11111110); // Only Four
    }

    #[test]
    fn ghosting_disappears_when_other_same_row_key_released() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::Six);

        assert_eq!(keyboard.read_row(6), 0b11111100);

        // Release Four — rectangle broken, Three and Six are in different rows
        // and different columns, no rectangle
        keyboard.release_key(&CpcKey::Four);

        assert_eq!(keyboard.read_row(6), 0b11111110); // Only Six at bit 0
        assert_eq!(keyboard.read_row(7), 0b11111101); // Only Three at bit 1
    }

    #[test]
    fn ghosting_multiple_rectangles_create_multiple_ghosts() {
        let mut keyboard = Keyboard::new();
        // Four (7, bit 0), Three (7, bit 1), X (7, bit 7), Six (6, bit 0)
        // Rectangle 1: Four, Three, Six → ghost at (6, bit 1)
        // Rectangle 2: Four, X, Six     → ghost at (6, bit 7)
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::X);
        keyboard.press_key(&CpcKey::Six);

        // Row 7: Four(bit 0), Three(bit 1), X(bit 7) → 0b01111100
        assert_eq!(keyboard.read_row(7), 0b01111100);
        // Row 6: Six(bit 0) + ghost(bit 1) + ghost(bit 7) → 0b01111100
        assert_eq!(keyboard.read_row(6), 0b01111100);
    }

    #[test]
    fn ghosting_partial_release_keeps_remaining_ghost() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::X);
        keyboard.press_key(&CpcKey::Six);

        // Two ghosts present
        assert_eq!(keyboard.read_row(6), 0b01111100);

        // Release Three — removes rectangle 1, keeps rectangle 2
        keyboard.release_key(&CpcKey::Three);

        // Row 7: Four(bit 0) + X(bit 7) → 0b01111110
        assert_eq!(keyboard.read_row(7), 0b01111110);
        // Row 6: Six(bit 0) + ghost(bit 7 from X) → 0b01111110
        assert_eq!(keyboard.read_row(6), 0b01111110);
    }

    #[test]
    fn ghosting_is_pressed_only_returns_actual_presses() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::Six);

        // Even with ghosting, is_pressed should only reflect actual presses
        let pressed: Vec<_> = CpcKey::ALL_KEYS
            .iter()
            .filter(|k| keyboard.is_pressed(k))
            .copied()
            .collect();
        assert_eq!(pressed.len(), 3);
        assert!(pressed.contains(&CpcKey::Four));
        assert!(pressed.contains(&CpcKey::Three));
        assert!(pressed.contains(&CpcKey::Six));
    }

    #[test]
    fn ghosting_pressed_keys_count_excludes_ghosts() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::Six);

        assert_eq!(keyboard.pressed_keys_count(), 3);
    }

    #[test]
    fn ghosting_reset_clears_all_ghosts() {
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Four);
        keyboard.press_key(&CpcKey::Three);
        keyboard.press_key(&CpcKey::Six);

        // Ghost present
        assert_eq!(keyboard.read_row(6), 0b11111100);

        keyboard.reset();

        for row in 0..10 {
            assert_eq!(
                keyboard.read_row(row),
                0xFF,
                "Row {} should be clear after reset",
                row
            );
        }
    }

    #[test]
    fn ghosting_ghost_bit_position_matches_fourth_corner() {
        // Verify the ghost appears at the exact bit position corresponding
        // to the fourth corner of the rectangle
        let mut keyboard = Keyboard::new();
        keyboard.press_key(&CpcKey::Four); // (7, bit 0)
        keyboard.press_key(&CpcKey::Three); // (7, bit 1)
        keyboard.press_key(&CpcKey::Six); // (6, bit 0)

        let row6 = keyboard.read_row(6);
        // Bit 0 cleared (Six pressed)
        assert_eq!(row6 & (1 << 0), 0, "Bit 0 should be cleared (Six pressed)");
        // Bit 1 cleared (ghost at fourth corner)
        assert_eq!(row6 & (1 << 1), 0, "Bit 1 should be cleared (ghost)");
        // Bit 2 should NOT be cleared (no ghost there)
        assert_ne!(row6 & (1 << 2), 0, "Bit 2 should not be cleared");
    }
}
