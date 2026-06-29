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

        if data[EXPECTED_HEADER.len()] != 1 || data[EXPECTED_HEADER.len() + 1] > 20 {
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
                    let flag = data[idx];
                    let pulses = if flag < 128 { 8063 } else { 3223 };

                    add_data_block(
                        data,
                        &mut idx,
                        &mut durations,
                        2168,
                        667,
                        735,
                        855,
                        1710,
                        pulses,
                        8,
                        bytes_to_follow,
                        pause,
                    );
                    idx += bytes_to_follow;
                }
                0x11 => {
                    if idx + 16 > data.len() {
                        return Err(TapeError::Truncated);
                    }
                    let pilot = (data[idx] as u64) | ((data[idx + 1] as u64) << 8);
                    idx += 2;
                    let sync1 = (data[idx] as u64) | ((data[idx + 1] as u64) << 8);
                    idx += 2;
                    let sync2 = (data[idx] as u64) | ((data[idx + 1] as u64) << 8);
                    idx += 2;
                    let zero = (data[idx] as u64) | ((data[idx + 1] as u64) << 8);
                    idx += 2;
                    let one = (data[idx] as u64) | ((data[idx + 1] as u64) << 8);
                    idx += 2;
                    let pulses = (data[idx] as u16) | ((data[idx + 1] as u16) << 8);
                    idx += 2;
                    let used_bits = data[idx];
                    idx += 1;
                    let pause = (data[idx] as u64) | ((data[idx + 1] as u64) << 8);
                    idx += 2;
                    let bytes_to_follow = (data[idx] as usize)
                        | ((data[idx + 1] as usize) << 8)
                        | ((data[idx + 2] as usize) << 16);
                    idx += 3;
                    if idx + bytes_to_follow > data.len() {
                        return Err(TapeError::Truncated);
                    }

                    add_data_block(
                        data,
                        &mut idx,
                        &mut durations,
                        pilot,
                        sync1,
                        sync2,
                        zero,
                        one,
                        pulses,
                        used_bits,
                        bytes_to_follow,
                        pause,
                    );
                    idx += bytes_to_follow;
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
                0x30 => {
                    if idx + 1 > data.len() {
                        return Err(TapeError::Truncated);
                    }
                    idx += 1 + data[idx] as usize;
                    if idx > data.len() {
                        return Err(TapeError::Truncated);
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

fn add_data_block(
    data: &[u8],
    idx: &mut usize,
    durations: &mut VecDeque<Pulse>,
    pilot: u64,
    sync1: u64,
    sync2: u64,
    zero: u64,
    one: u64,
    pulses: u16,
    used_bits: u8,
    length: usize,
    pause: u64,
) {
    for _ in 0..pulses {
        durations.push_back(Pulse::new_toggle(pilot));
    }
    durations.push_back(Pulse::new_toggle(sync1));
    durations.push_back(Pulse::new_toggle(sync2));
    let block = &data[*idx..*idx + length];
    for (counter, byte) in block.iter().enumerate() {
        let used = if counter + 1 == block.len() {
            used_bits
        } else {
            8
        };
        for bit in ((8 - used)..8).rev() {
            let p = if ((byte >> bit) & 1) != 0 { one } else { zero };
            durations.push_back(Pulse::new_toggle(p));
            durations.push_back(Pulse::new_toggle(p));
        }
    }

    if pause > 0 {
        durations.push_back(Pulse::new_low(3500 * pause));
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
    fn from_cdt_accepts_standard_v1_header() {
        for minor in 0..=20 {
            // Tests that we don't hardcode v1.13
            let header = make_cdt_header(1, minor);
            let mut data = header.clone();
            data.extend_from_slice(&[0x20, 0x01, 0x00]);
            let result = TapePlayer::from_cdt(&data);
            assert!(result.is_ok(), "v1.20 header must be accepted");
        }
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

    fn make_cdt_with_block_30(text: &[u8]) -> Vec<u8> {
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        v.push(text.len() as u8);
        v.extend_from_slice(text);
        v
    }

    #[test]
    fn block_30_parses_without_error() {
        let cdt = make_cdt_with_block_30(b"Level 1");
        let result = TapePlayer::from_cdt(&cdt);
        assert!(result.is_ok(), "Block 30 must parse without error");
    }

    #[test]
    fn block_30_generates_no_pulses() {
        let cdt = make_cdt_with_block_30(b"Level 1");
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        assert!(
            player.durations.is_empty(),
            "Block 30 must not produce any pulses"
        );
    }

    #[test]
    fn block_30_zero_length_text_is_valid() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        v.push(0x00); // N = 0
        let result = TapePlayer::from_cdt(&v);
        assert!(result.is_ok(), "Zero-length text description must be valid");
    }

    #[test]
    fn block_30_max_length_255_is_valid() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        v.push(255);
        v.extend_from_slice(&vec![b'A'; 255]);
        let result = TapePlayer::from_cdt(&v);
        assert!(
            result.is_ok(),
            "Max length (255) text description must be valid"
        );
    }

    #[test]
    fn block_30_truncated_text_returns_truncated_error() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        v.push(10); // claims 10 bytes
        v.extend_from_slice(b"Short"); // only 5 bytes follow
        let result = TapePlayer::from_cdt(&v);
        assert!(
            matches!(result, Err(TapeError::Truncated)),
            "Truncated block 30 text must return Truncated error"
        );
    }

    #[test]
    fn block_30_missing_length_byte_returns_truncated_error() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        // No length byte at all
        let result = TapePlayer::from_cdt(&v);
        assert!(
            matches!(result, Err(TapeError::Truncated)),
            "Block 30 without length byte must return Truncated error"
        );
    }

    #[test]
    fn block_30_multiple_in_sequence_produce_no_pulses() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        v.push(5);
        v.extend_from_slice(b"Hello");
        v.push(0x30);
        v.push(5);
        v.extend_from_slice(b"World");
        let player = TapePlayer::from_cdt(&v).unwrap();
        assert!(
            player.durations.is_empty(),
            "Multiple block 30s must not produce any pulses"
        );
    }

    #[test]
    fn block_30_followed_by_block_20_preserves_pause() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        v.push(5);
        v.extend_from_slice(b"Hello");
        v.extend_from_slice(&[0x20, 0x01, 0x00]); // 1ms pause
        let player = TapePlayer::from_cdt(&v).unwrap();
        assert_eq!(
            player.durations.len(),
            1,
            "Block 30 should be skipped; only the block 20 pause should exist"
        );
    }

    #[test]
    fn block_30_preceding_block_20_preserves_pause() {
        let mut v = make_cdt_header(1, 13);
        v.extend_from_slice(&[0x20, 0x01, 0x00]); // 1ms pause
        v.push(0x30);
        v.push(5);
        v.extend_from_slice(b"Hello");
        let player = TapePlayer::from_cdt(&v).unwrap();
        assert_eq!(
            player.durations.len(),
            1,
            "Block 20 pause before block 30 should still exist"
        );
    }

    #[test]
    fn block_30_between_two_block_20s_preserves_both_pauses() {
        let mut v = make_cdt_header(1, 13);
        v.extend_from_slice(&[0x20, 0x01, 0x00]); // 1ms pause
        v.push(0x30);
        v.push(7);
        v.extend_from_slice(b"Level 1");
        v.extend_from_slice(&[0x20, 0x02, 0x00]); // 2ms pause
        let player = TapePlayer::from_cdt(&v).unwrap();
        assert_eq!(
            player.durations.len(),
            2,
            "Block 30 between two block 20s must not consume or alter either pause"
        );
    }

    #[test]
    fn block_30_does_not_affect_ear_state() {
        let cdt = make_cdt_with_block_30(b"Level 1");
        let mut player = TapePlayer::from_cdt(&cdt).unwrap();
        player.play();
        player.advance(1_000_000);
        assert!(
            !player.ear(),
            "Block 30 must not toggle or affect EAR state"
        );
    }

    #[test]
    fn block_30_alone_results_in_non_playing_tape() {
        let cdt = make_cdt_with_block_30(b"Level 1");
        let mut player = TapePlayer::from_cdt(&cdt).unwrap();
        player.play();
        assert!(
            !player.is_playing(),
            "Tape with only block 30 should not be playing (no pulses to consume)"
        );
    }

    #[test]
    fn block_30_with_carriage_return_in_text_is_valid() {
        // The CDT spec uses 0x0D as a line separator within text fields
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        v.push(15);
        v.extend_from_slice(b"Level 1\x0DLevel 2");
        let result = TapePlayer::from_cdt(&v);
        assert!(
            result.is_ok(),
            "Block 30 with 0x0D line separator must be valid"
        );
    }

    #[test]
    fn block_30_with_extended_latin1_characters_is_valid() {
        // ISO 8859-1 (Latin-1) encoding is permitted
        let mut v = make_cdt_header(1, 13);
        v.push(0x30);
        v.push(3);
        v.extend_from_slice(&[0xC3, 0xA9, 0xFC]); // Latin-1 bytes
        let result = TapePlayer::from_cdt(&v);
        assert!(
            result.is_ok(),
            "Block 30 with Latin-1 characters must be valid"
        );
    }

    #[test]
    fn block_30_as_first_block_followed_by_block_10_works() {
        let mut v = make_cdt_header(1, 13);
        // Text description first
        v.push(0x30);
        v.push(9);
        v.extend_from_slice(b"Main Menu");
        // Then a standard data block with 2-byte payload
        v.push(0x10);
        v.push(0x01);
        v.push(0x00);
        v.push(0x02);
        v.push(0x00);
        v.extend_from_slice(&[0x00, 0x00]);

        let player = TapePlayer::from_cdt(&v).unwrap();
        // Block 10 with flag < 128: 8063 pilot + 2 sync + 2 bytes * 16 pulses + 1 pause
        assert_eq!(
            player.durations.len(),
            8063 + 2 + 32 + 1,
            "Block 30 before block 10 must not alter block 10 pulse count"
        );
    }

    #[test]
    fn block_30_as_last_block_does_not_error() {
        let mut v = make_cdt_header(1, 13);
        v.extend_from_slice(&[0x20, 0x01, 0x00]); // 1ms pause
        v.push(0x30);
        v.push(9);
        v.extend_from_slice(b"Game Over");

        let player = TapePlayer::from_cdt(&v).unwrap();
        assert_eq!(
            player.durations.len(),
            1,
            "Block 30 as last block must not add pulses"
        );
    }

    fn make_cdt_with_block_11(
        pilot: u16,
        sync1: u16,
        sync2: u16,
        zero: u16,
        one: u16,
        pilot_tone: u16,
        used_bits: u8,
        pause: u16,
        data: &[u8],
    ) -> Vec<u8> {
        let mut v = make_cdt_header(1, 13);
        v.push(0x11);
        v.extend_from_slice(&pilot.to_le_bytes());
        v.extend_from_slice(&sync1.to_le_bytes());
        v.extend_from_slice(&sync2.to_le_bytes());
        v.extend_from_slice(&zero.to_le_bytes());
        v.extend_from_slice(&one.to_le_bytes());
        v.extend_from_slice(&pilot_tone.to_le_bytes());
        v.extend_from_slice(&used_bits.to_le_bytes());
        v.extend_from_slice(&pause.to_le_bytes());
        // 3-byte length
        v.push((data.len() & 0xFF) as u8);
        v.push(((data.len() >> 8) & 0xFF) as u8);
        v.push(((data.len() >> 16) & 0xFF) as u8);
        v.extend_from_slice(data);
        v
    }

    #[test]
    fn block_11_parses_without_error() {
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 10, 8, 0, &[0xFF]);
        assert!(TapePlayer::from_cdt(&cdt).is_ok());
    }

    #[test]
    fn block_11_generates_correct_pulse_count() {
        // 10 pilot + 2 sync + 1 byte * 8 bits * 2 pulses = 10 + 2 + 16 = 28
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 10, 8, 0, &[0xFF]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        assert_eq!(player.durations.len(), 28);
    }

    #[test]
    fn block_11_includes_pause_pulse_if_pause_nonzero() {
        // 10 + 2 + 16 = 28 data pulses + 1 pause = 29
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 10, 8, 10, &[0xFF]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        assert_eq!(player.durations.len(), 29);
        assert!(matches!(player.durations.back().unwrap(), Pulse::Low(_)));
    }

    #[test]
    fn block_11_zero_pause_does_not_add_pause_pulse() {
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 10, 8, 0, &[0xFF]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        assert_eq!(player.durations.len(), 28);
        assert!(!matches!(player.durations.back().unwrap(), Pulse::Low(_)));
    }

    #[test]
    fn block_11_pilot_pulses_use_specified_duration() {
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 5, 8, 0, &[0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        for i in 0..5 {
            assert_eq!(player.durations[i], Pulse::new_toggle(1000));
        }
    }

    #[test]
    fn block_11_sync_pulses_use_specified_durations() {
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 5, 8, 0, &[0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        assert_eq!(player.durations[5], Pulse::new_toggle(2000));
        assert_eq!(player.durations[6], Pulse::new_toggle(3000));
    }

    #[test]
    fn block_11_bit_0_uses_zero_bit_duration() {
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 5, 8, 0, &[0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        let data_start = 7; // 5 pilot + 2 sync
        for i in 0..16 {
            assert_eq!(player.durations[data_start + i], Pulse::new_toggle(400));
        }
    }

    #[test]
    fn block_11_bit_1_uses_one_bit_duration() {
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 5, 8, 0, &[0xFF]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        let data_start = 7;
        for i in 0..16 {
            assert_eq!(player.durations[data_start + i], Pulse::new_toggle(800));
        }
    }

    #[test]
    fn block_11_used_bits_limits_last_byte_bits() {
        // used_bits = 3, data = 0b10000000 -> only 3 bits processed
        // Bits 7, 6, 5 are processed.
        // Bit 7 = 1 (two 800 pulses). Bits 6, 5 = 0 (four 400 pulses).
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 5, 3, 0, &[0b10000000]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        // Total: 5 pilot + 2 sync + 6 data = 13 pulses
        assert_eq!(player.durations.len(), 13);

        let data_start = 7;
        // Bit 7 is 1
        assert_eq!(player.durations[data_start], Pulse::new_toggle(800));
        assert_eq!(player.durations[data_start + 1], Pulse::new_toggle(800));
        // Bits 6 and 5 are 0
        assert_eq!(player.durations[data_start + 2], Pulse::new_toggle(400));
        assert_eq!(player.durations[data_start + 3], Pulse::new_toggle(400));
        assert_eq!(player.durations[data_start + 4], Pulse::new_toggle(400));
        assert_eq!(player.durations[data_start + 5], Pulse::new_toggle(400));
    }

    #[test]
    fn block_11_multi_byte_data_preserves_byte_order() {
        // data: 0xFF, 0x00
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 5, 8, 0, &[0xFF, 0x00]);
        let player = TapePlayer::from_cdt(&cdt).unwrap();

        let data_start = 7;
        // Byte 0 (0xFF): 16 pulses of 800
        for i in 0..16 {
            assert_eq!(player.durations[data_start + i], Pulse::new_toggle(800));
        }
        // Byte 1 (0x00): 16 pulses of 400
        for i in 16..32 {
            assert_eq!(player.durations[data_start + i], Pulse::new_toggle(400));
        }
    }

    #[test]
    fn block_11_rejects_truncated_header() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x11);
        v.extend_from_slice(&[0x00, 0x00, 0x00]); // Only 3 bytes instead of 19
        let result = TapePlayer::from_cdt(&v);
        assert!(matches!(result, Err(TapeError::Truncated)));
    }

    #[test]
    fn block_11_rejects_truncated_data() {
        let mut v = make_cdt_header(1, 13);
        v.push(0x11);
        // Minimal valid header (16 bytes)
        v.extend_from_slice(&[0u8; 15]);
        v.push(10); // claims 10 bytes follow
        v.extend_from_slice(&[0u8; 5]); // only 5 follow
        let result = TapePlayer::from_cdt(&v);
        assert!(matches!(result, Err(TapeError::Truncated)));
    }

    #[test]
    fn block_11_supports_large_data_length_3_bytes() {
        let data = vec![0xAA; 300];
        // 1 byte = 16 pulses. 300 bytes = 4800 pulses.
        // 5 pilot + 2 sync + 4800 = 4807
        let cdt = make_cdt_with_block_11(1000, 2000, 3000, 400, 800, 5, 8, 0, &data);
        let player = TapePlayer::from_cdt(&cdt).unwrap();
        assert_eq!(player.durations.len(), 4807);
    }
}
