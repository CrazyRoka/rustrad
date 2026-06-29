use std::collections::VecDeque;

const ZX_SCALE: u64 = 7;
const CPC_SCALE: u64 = 8;

#[derive(Debug)]
pub enum TapeError {
    InvalidHeader,
    UnknownBlockId(u8),
    Truncated,
}

#[derive(Debug, PartialEq, Eq)]
enum Pulse {
    Toggle(u64),
    Low(u64),
    High(u64),
}

impl Pulse {
    fn new_toggle(cycles: u64) -> Self {
        Self::Toggle(cycles * CPC_SCALE)
    }

    fn new_low(cycles: u64) -> Self {
        Self::Low(cycles * CPC_SCALE)
    }

    fn new_high(cycles: u64) -> Self {
        Self::High(cycles * CPC_SCALE)
    }

    fn duration(&self) -> u64 {
        match self {
            Pulse::Toggle(x) => *x,
            Pulse::Low(x) => *x,
            Pulse::High(x) => *x,
        }
    }

    fn duration_mut(&mut self) -> &mut u64 {
        match self {
            Pulse::Toggle(x) => x,
            Pulse::Low(x) => x,
            Pulse::High(x) => x,
        }
    }
}

#[derive(Debug)]
pub struct TapePlayer {
    durations: VecDeque<Pulse>,
    ear: bool,
    playing: bool,
}

impl TapePlayer {
    pub fn from_cdt(data: &[u8]) -> Result<Self, TapeError> {
        const EXPECTED_HEADER: &[u8] = b"ZXTape!\x1a";

        if data.len() < EXPECTED_HEADER.len() + 2 {
            return Err(TapeError::Truncated);
        }

        if !data.starts_with(EXPECTED_HEADER) {
            return Err(TapeError::InvalidHeader);
        }

        if data[EXPECTED_HEADER.len()] != 1
            || (data[EXPECTED_HEADER.len() + 1] != 13 && data[EXPECTED_HEADER.len() + 1] != 20)
        {
            return Err(TapeError::InvalidHeader);
        }

        let mut durations = VecDeque::new();
        let mut idx = EXPECTED_HEADER.len() + 2;
        while idx < data.len() {
            let block_type = data[idx];
            idx += 1;

            match block_type {
                0x10 => {
                    if idx + 4 > data.len() {
                        return Err(TapeError::Truncated);
                    }
                    let pause_low = data[idx];
                    let pause_high = data[idx + 1];
                    let pause = (pause_low as u64) | ((pause_high as u64) << 8);
                    idx += 2;

                    let low = data[idx] as usize;
                    let high = data[idx + 1] as usize;
                    idx += 2;

                    let bytes_to_follow = (high << 8) | low;
                    if idx + bytes_to_follow > data.len() {
                        return Err(TapeError::Truncated);
                    }

                    let block = &data[idx..idx + bytes_to_follow];
                    idx += bytes_to_follow;

                    let flag = block[0];
                    let pulses = if flag < 128 { 8063 } else { 3223 };

                    for _ in 0..pulses {
                        durations.push_back(Pulse::new_toggle(2168));
                    }
                    durations.push_back(Pulse::new_toggle(667));
                    durations.push_back(Pulse::new_toggle(735));

                    for byte in block {
                        for bit in (0..8).rev() {
                            let p = if ((byte >> bit) & 1) != 0 { 1710 } else { 855 };
                            durations.push_back(Pulse::new_toggle(p));
                            durations.push_back(Pulse::new_toggle(p));
                        }
                    }

                    if pause > 0 {
                        durations.push_back(Pulse::new_low(3500 * pause));
                    }
                }
                0x20 => {
                    if idx + 2 > data.len() {
                        return Err(TapeError::Truncated);
                    }

                    let pause_low = data[idx];
                    let pause_high = data[idx + 1];
                    let pause = (pause_low as u64) | ((pause_high as u64) << 8);
                    idx += 2;

                    if pause > 0 {
                        durations.push_back(Pulse::new_low(3500 * pause));
                    }
                }
                _ => return Err(TapeError::UnknownBlockId(block_type)),
            }
        }

        Ok(Self {
            durations,
            ear: false,
            playing: false,
        })
    }

    pub fn advance(&mut self, mut cycles: u64) {
        if !self.is_playing() {
            return;
        }
        cycles *= ZX_SCALE;

        while cycles > 0 && !self.durations.is_empty() {
            let pulse = self.durations.front_mut().expect("Data should be there");
            if pulse.duration() > cycles {
                *pulse.duration_mut() -= cycles;
                break;
            } else {
                cycles -= pulse.duration();
                self.ear = match pulse {
                    Pulse::Toggle(_) => !self.ear,
                    Pulse::Low(_) => false,
                    Pulse::High(_) => true,
                };
                self.durations.pop_front();
            }
        }
    }

    pub fn ear(&self) -> bool {
        self.ear
    }

    pub fn is_playing(&self) -> bool {
        self.playing && !self.durations.is_empty()
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let tape_data = make_cdt_with_block_10(1, &[0]);
        let player = TapePlayer::from_cdt(&tape_data).unwrap();

        assert!(
            !player.is_playing(),
            "Player should not start playing automatically"
        );
        assert!(!player.ear, "EAR should be FALSE initially");
    }

    #[test]
    fn test_play_and_stop_state_transitions() {
        let tape_data = make_cdt_with_block_10(1, &[0]);
        let mut player = TapePlayer::from_cdt(&tape_data).unwrap();

        player.play();
        assert!(
            player.is_playing(),
            "Player should start playing when requested"
        );

        player.stop();
        assert!(
            !player.is_playing(),
            "Player should stop playing when requested"
        );
    }

    #[test]
    fn test_empty_tape_is_never_playing() {
        let tape_data = make_cdt_with_block_10(1, &[0]);
        let mut player = TapePlayer::from_cdt(&tape_data).unwrap();

        player.play();
        assert!(player.is_playing());

        player.durations.clear();
        assert!(
            !player.is_playing(),
            "Player should stop playing when empty"
        );
    }

    #[test]
    fn test_advance_before_play() {
        let tape_data = make_cdt_with_block_10(1, &[0]);
        let mut player = TapePlayer::from_cdt(&tape_data).unwrap();
        let initial_ear = player.ear();

        player.advance(10_000_000);

        assert!(!player.is_playing(), "Player should remain stopped");
        assert_eq!(
            player.ear(),
            initial_ear,
            "EAR should not change while stopped"
        );
    }

    #[test]
    fn test_empty_tape_does_not_play() {
        let tape_data = make_cdt_header(1, 13);
        let mut player = TapePlayer::from_cdt(&tape_data).unwrap();

        player.play();
        assert!(
            !player.is_playing(),
            "An empty tape should not transition to playing"
        );
    }

    fn make_cdt_header(major: u8, minor: u8) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"ZXTape!\x1a");
        v.push(major);
        v.push(minor);
        v
    }

    #[test]
    fn from_cdt_rejects_empty_data() {
        let result = TapePlayer::from_cdt(&[]);
        assert!(matches!(result, Err(TapeError::Truncated)));
    }

    #[test]
    fn from_cdt_rejects_invalid_signature() {
        let mut bad = Vec::new();
        bad.extend_from_slice(b"NOTTape!");
        bad.push(1);
        bad.push(13);
        bad.extend_from_slice(&[0x20, 0x00, 0x00]);
        let result = TapePlayer::from_cdt(&bad);
        assert!(matches!(result, Err(TapeError::InvalidHeader)));
    }

    #[test]
    fn from_cdt_rejects_truncated_header() {
        let result = TapePlayer::from_cdt(b"ZXTape");
        assert!(matches!(result, Err(TapeError::Truncated)));
    }

    #[test]
    fn from_cdt_accepts_standard_v1_13_header() {
        let header = make_cdt_header(1, 13);
        let mut data = header.clone();
        data.extend_from_slice(&[0x20, 0x01, 0x00]);
        let result = TapePlayer::from_cdt(&data);
        assert!(result.is_ok(), "v1.13 header must be accepted");
    }

    #[test]
    fn from_cdt_accepts_v1_20_header() {
        // Tests that we don't hardcode v1.13
        let header = make_cdt_header(1, 20);
        let mut data = header.clone();
        data.extend_from_slice(&[0x20, 0x01, 0x00]);
        let result = TapePlayer::from_cdt(&data);
        assert!(result.is_ok(), "v1.20 header must be accepted");
    }

    #[test]
    fn from_cdt_rejects_truncated_block_10() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x10);
        v.push(0x01); // pause
        v.push(0x00);
        v.push(0xFF); // length = 255
        v.push(0x00);
        // Missing actual payload bytes
        let result = TapePlayer::from_cdt(&v);
        assert!(matches!(result, Err(TapeError::Truncated)));
    }

    #[test]
    fn from_cdt_rejects_unknown_block_id() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x99); // Unknown block ID
        let result = TapePlayer::from_cdt(&v);
        assert!(matches!(result, Err(TapeError::UnknownBlockId(0x99))));
    }

    fn make_cdt_with_block_10(pause_ms: u16, payload: &[u8]) -> Vec<u8> {
        let mut v = make_cdt_header(1, 13);
        v.push(0x10);
        v.push((pause_ms & 0xFF) as u8);
        v.push((pause_ms >> 8) as u8);
        v.push((payload.len() & 0xFF) as u8);
        v.push((payload.len() >> 8) as u8);
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn block_20_zero_pause_is_ignored_on_cpc() {
        let mut v = make_cdt_header(1, 13);
        v.extend_from_slice(&[0x20, 0x00, 0x00]); // pause 0
        v.extend_from_slice(&[0x20, 0x01, 0x00]); // pause 1 ms
        let mut p = TapePlayer::from_cdt(&v).unwrap();
        p.play();
        p.advance(1_000);
        assert!(p.is_playing(), "Pause 0 must NOT stop the tape on CPC");

        // Verify no zero-duration pulse was added
        let p2 = TapePlayer::from_cdt(&v).unwrap();
        assert_eq!(p2.durations.len(), 1); // Only the 1ms pause should exist
    }

    #[test]
    fn block_20_nonzero_pause_outputs_low_for_duration() {
        let mut v = make_cdt_header(1, 13);
        v.extend_from_slice(&[0x20, 0x02, 0x00]); // pause 2 ms
        let mut p = TapePlayer::from_cdt(&v).unwrap();
        p.play();
        p.advance(1);
        assert!(!p.ear(), "EAR must be LOW (false) during pause block");
    }

    #[test]
    fn block_20_pause_terminates_tape_when_last_block() {
        let mut v = make_cdt_header(1, 13);
        v.extend_from_slice(&[0x20, 0x05, 0x00]); // 5 ms
        let mut p = TapePlayer::from_cdt(&v).unwrap();
        p.play();
        p.advance(20_000);
        assert!(
            !p.is_playing(),
            "Player must stop after final pause expires"
        );
        // Edge: 1 cycle before end still playing
        let mut p2 = TapePlayer::from_cdt(&v).unwrap();
        p2.play();
        p2.advance(19_999);
        assert!(p2.is_playing());
    }

    #[test]
    fn block_10_flag_less_than_128_generates_8063_pilot_pulses() {
        // Testing flag 0x01 (which is < 128) to ensure it doesn't panic
        let cdt = make_cdt_with_block_10(1, &[0x01, 0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        // 8063 pilot + 2 sync + 2 bytes * 8 bits * 2 pulses + 1 pause = 8098
        assert_eq!(player.durations.len(), 8098);

        for i in 0..8063 {
            assert_eq!(player.durations[i], Pulse::new_toggle(2168));
        }
    }

    #[test]
    fn block_10_flag_greater_than_127_generates_3223_pilot_pulses() {
        // Testing flag 0xFE (which is >= 128) to ensure it doesn't panic
        let cdt = make_cdt_with_block_10(1, &[0xFE, 0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        // 3223 pilot + 2 sync + 2 bytes * 8 bits * 2 pulses + 1 pause = 3258
        assert_eq!(player.durations.len(), 3258);

        for i in 0..3223 {
            assert_eq!(player.durations[i], Pulse::new_toggle(2168));
        }
    }

    #[test]
    fn block_10_polarity_forces_low_on_pause() {
        // Ensure the pause at the end of block 10 is Pulse::Low, not Toggle
        let cdt = make_cdt_with_block_10(10, &[0xFF, 0xFF]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let last_pulse = player.durations.back().unwrap();
        assert!(
            matches!(last_pulse, Pulse::Low(_)),
            "Final pause must be Pulse::Low"
        );
    }

    #[test]
    fn block_10_zero_pause_is_not_pushed() {
        let cdt = make_cdt_with_block_10(0, &[0x00, 0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let last_pulse = player.durations.back().unwrap();
        // The last pulse should be the final data bit, not a zero-duration pause
        assert!(
            matches!(last_pulse, Pulse::Toggle(_)),
            "Zero pause should not be added to durations"
        );
    }

    #[test]
    fn block_10_sync2_pulse_has_correct_duration_and_polarity() {
        let cdt = make_cdt_with_block_10(1, &[0x00, 0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        match &player.durations[8064] {
            Pulse::Toggle(d) => assert_eq!(*d, 735 * CPC_SCALE),
            other => panic!("Sync2 must be Toggle, got {:?}", other),
        }
    }

    #[test]
    fn block_10_bit_0_produces_two_pulses_of_855() {
        let cdt = make_cdt_with_block_10(1, &[0x00, 0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let data_start = 8065;
        for i in 0..32 {
            assert_eq!(player.durations[data_start + i], Pulse::new_toggle(855));
        }
    }

    #[test]
    fn block_10_bit_1_produces_two_pulses_of_1710() {
        let cdt = make_cdt_with_block_10(1, &[0xFF, 0xFF]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let data_start = 3225;
        for i in 0..32 {
            assert_eq!(player.durations[data_start + i], Pulse::new_toggle(1710));
        }
    }

    #[test]
    fn block_10_each_bit_produces_exactly_two_pulses_of_same_duration() {
        let cdt = make_cdt_with_block_10(1, &[0x00, 0xAA]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let data_start = 8065;
        for bit_idx in 0..16 {
            let p1 = player.durations[data_start + bit_idx * 2].duration();
            let p2 = player.durations[data_start + bit_idx * 2 + 1].duration();
            assert_eq!(
                p1, p2,
                "Bit {} must produce 2 pulses of identical duration",
                bit_idx
            );
        }
    }

    #[test]
    fn block_10_byte_0xaa_encodes_bits_msb_first() {
        // 0xAA = 0b10101010
        // MSB first: 1,0,1,0,1,0,1,0
        let cdt = make_cdt_with_block_10(1, &[0xFF, 0xAA]); // flag 0xFF, data 0xAA
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let data_start = 3225; // 3223 pilot + 2 sync
        // Skip flag byte (0xFF = all 1s, 16 pulses)
        let byte_start = data_start + 16;

        let expected_bits: [u64; 8] = [1, 0, 1, 0, 1, 0, 1, 0];
        for (bit_idx, &bit) in expected_bits.iter().enumerate() {
            let expected_dur = if bit == 1 {
                1710 * CPC_SCALE
            } else {
                855 * CPC_SCALE
            };
            for half in 0..2 {
                let pulse_idx = byte_start + bit_idx * 2 + half;
                match &player.durations[pulse_idx] {
                    Pulse::Toggle(d) => assert_eq!(
                        *d, expected_dur,
                        "0xAA bit {} half {} must have duration for bit value {}",
                        bit_idx, half, bit
                    ),
                    other => panic!("Must be Toggle, got {:?}", other),
                }
            }
        }
    }

    #[test]
    fn block_10_byte_0x80_encodes_single_msb_1_then_seven_0s() {
        // 0x80 = 0b10000000
        let cdt = make_cdt_with_block_10(1, &[0xFF, 0x80]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let data_start = 3225;
        let byte_start = data_start + 16; // skip flag byte

        // Bit 7 (MSB) = 1
        for half in 0..2 {
            match &player.durations[byte_start + half] {
                Pulse::Toggle(d) => assert_eq!(*d, 1710 * CPC_SCALE, "MSB must be 1"),
                other => panic!("{:?}", other),
            }
        }

        // Bits 6..0 = 0
        for bit_idx in 1..8 {
            for half in 0..2 {
                match &player.durations[byte_start + bit_idx * 2 + half] {
                    Pulse::Toggle(d) => {
                        assert_eq!(*d, 855 * CPC_SCALE, "Bit {} of 0x80 must be 0", 7 - bit_idx)
                    }
                    other => panic!("{:?}", other),
                }
            }
        }
    }

    #[test]
    fn block_10_multi_byte_data_preserves_byte_order() {
        // Flag 0x00, then bytes 0xFF, 0x00, 0xAA
        let cdt = make_cdt_with_block_10(1, &[0x00, 0xFF, 0x00, 0xAA]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let data_start = 8065;
        let ppb = 16; // pulses per byte

        // Byte 0 (flag 0x00): all 0 bits
        for i in 0..ppb {
            assert_eq!(player.durations[data_start + i].duration(), 855 * CPC_SCALE);
        }
        // Byte 1 (0xFF): all 1 bits
        for i in 0..ppb {
            assert_eq!(
                player.durations[data_start + ppb + i].duration(),
                1710 * CPC_SCALE
            );
        }
        // Byte 2 (0x00): all 0 bits
        for i in 0..ppb {
            assert_eq!(
                player.durations[data_start + ppb * 2 + i].duration(),
                855 * CPC_SCALE
            );
        }
        // Byte 3 (0xAA): alternating 1,0
        let expected = [1710, 855, 1710, 855, 1710, 855, 1710, 855];
        for (bit_idx, &dur) in expected.iter().enumerate() {
            for half in 0..2 {
                assert_eq!(
                    player.durations[data_start + ppb * 3 + bit_idx * 2 + half].duration(),
                    dur * CPC_SCALE
                );
            }
        }
    }

    #[test]
    fn block_10_total_pulse_count_matches_expected() {
        // Flag 0x00, 3 data bytes, pause 10ms
        let cdt = make_cdt_with_block_10(10, &[0x00, 0xAA, 0x55, 0xFF]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        // 8063 pilot + 2 sync + 4 bytes * 8 bits * 2 pulses + 1 pause
        assert_eq!(player.durations.len(), 8063 + 2 + 64 + 1);
    }

    #[test]
    fn block_10_pilot_pulse_consumes_exactly_2168_cdt_t_states() {
        let cdt = make_cdt_with_block_10(1, &[0x00, 0x00]);
        let mut p = TapePlayer::from_cdt(&cdt).unwrap();
        p.play();

        let initial_ear = p.ear();

        // Pilot = 2168 CDT T-states = 2168 * 8 = 17344 internal units
        // advance(X) consumes X * 7 internal units
        // Need X * 7 >= 17344 → X >= ceil(17344 / 7) = 2478
        // 2477 * 7 = 17339 < 17344 (not enough)
        // 2478 * 7 = 17346 >= 17344 (just enough)

        p.advance(2477);
        assert_eq!(
            p.ear(),
            initial_ear,
            "EAR must not toggle before pilot pulse boundary (2477 * 7 = 17339 < 17344)"
        );

        p.advance(1);
        assert_ne!(
            p.ear(),
            initial_ear,
            "EAR must toggle at pilot pulse boundary (2478 * 7 = 17346 >= 17344)"
        );
    }

    #[test]
    fn block_10_pilot_pulses_alternate_ear_state() {
        let cdt = make_cdt_with_block_10(1, &[0x00, 0x00]);
        let mut p = TapePlayer::from_cdt(&cdt).unwrap();
        p.play();

        let initial = p.ear();

        // 2478 CPC T-states per pilot pulse
        for i in 0..5 {
            p.advance(2478);
            let expected = if i % 2 == 0 { !initial } else { initial };
            assert_eq!(
                p.ear(),
                expected,
                "After pilot pulse {}, EAR should be {}",
                i + 1,
                expected
            );
        }
    }
}
