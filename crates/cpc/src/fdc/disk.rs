const STANDARD_HEADER: &[u8] = b"MV - CPCEMU";
const EXTENDED_HEADER: &[u8] = b"EXTENDED CPC DSK File";
const TRACK_HEADER: &[u8] = b"Track-Info\r\n";

const CREATOR: usize = 0x22;
const TRACKS: usize = 0x30;
const SIDES: usize = 0x31;
const TRACK_SIZE: usize = 0x32;
const TRACK_SIZE_TABLE: usize = 0x34;

const TRACK_NUMBER: usize = 0x10;
const SIDE_NUMBER: usize = 0x11;
const DATA_RATE: usize = 0x12;
const REC_MODE: usize = 0x13;
const SECTOR_SIZE: usize = 0x14;
const SECTOR_COUNT: usize = 0x15;
const GAP3: usize = 0x16;
const FILLER: usize = 0x17;
const SECTOR_INFO: usize = 0x18;
const SECTOR_INFO_SIZE: usize = 8;

#[derive(Debug, PartialEq, Eq)]
pub enum DiskError {
    InvalidHeader,
    InvalidTrackInfo,
    InvalidSectorInfo,
    Truncated,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Format {
    Standard,
    Extended,
}

#[derive(Debug, PartialEq, Eq)]
struct SectorInfo {
    track: u8,
    side: u8,
    id: u8,
    size: u8,
    st1: u8,
    st2: u8,
    data_length: u16,
    data: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
enum DataRate {
    Unknown,
    SdDd,
    Hd,
    Ed,
}

#[derive(Debug, PartialEq, Eq)]
enum RecordingMode {
    Unknown,
    Fm,
    Mfm,
}

#[derive(Debug, PartialEq, Eq)]
struct TrackInfo {
    track_size: u16,
    track_number: u8,
    side_number: u8,
    data_rate: DataRate,
    recording_mode: RecordingMode,
    sector_size: u8,
    sector_count: u8,
    gap3: u8,
    filler: u8,
    sectors: Vec<SectorInfo>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Disk {
    format: Format,
    creator: [u8; 14],
    track_count: u8,
    side_count: u8,
    tracks: Vec<Option<TrackInfo>>,
}

impl Disk {
    pub fn from_bytes(data: &[u8]) -> Result<Self, DiskError> {
        if data.len() < 256 {
            return Err(DiskError::Truncated);
        }

        let format = if data.starts_with(STANDARD_HEADER) {
            Format::Standard
        } else if data.starts_with(EXTENDED_HEADER) {
            Format::Extended
        } else {
            return Err(DiskError::InvalidHeader);
        };

        let mut creator = [0; 14];
        creator.copy_from_slice(&data[CREATOR..TRACKS]);
        let track_count = data[TRACKS];
        let side_count = data[SIDES];
        if side_count != 1 && side_count != 2 {
            return Err(DiskError::InvalidHeader);
        }
        let standard_track_size = (data[TRACK_SIZE] as u16) | ((data[TRACK_SIZE + 1] as u16) << 8);

        let mut tracks = Vec::with_capacity(track_count as usize * side_count as usize);
        let mut offset = 0x100;
        for track in 0..track_count {
            for side in 0..side_count {
                let track_size = if format == Format::Standard {
                    standard_track_size
                } else {
                    (data[TRACK_SIZE_TABLE + track as usize * side_count as usize + side as usize]
                        as u16)
                        << 8
                };
                if track_size == 0 {
                    tracks.push(None);
                    continue;
                }
                if data.len() < offset + track_size as usize {
                    return Err(DiskError::Truncated);
                }
                if !data[offset..].starts_with(TRACK_HEADER) {
                    return Err(DiskError::InvalidTrackInfo);
                }

                let track_number = data[offset + TRACK_NUMBER];
                let side_number = data[offset + SIDE_NUMBER];
                let data_rate = match data[offset + DATA_RATE] {
                    0 => DataRate::Unknown,
                    1 => DataRate::SdDd,
                    2 => DataRate::Hd,
                    3 => DataRate::Ed,
                    _ => return Err(DiskError::InvalidTrackInfo),
                };
                let recording_mode = match data[offset + REC_MODE] {
                    0 => RecordingMode::Unknown,
                    1 => RecordingMode::Fm,
                    2 => RecordingMode::Mfm,
                    _ => return Err(DiskError::InvalidTrackInfo),
                };
                let sector_size = data[offset + SECTOR_SIZE];
                let sector_count = data[offset + SECTOR_COUNT];
                let gap3 = data[offset + GAP3];
                let filler = data[offset + FILLER];
                let mut sectors = Vec::with_capacity(sector_count as usize);

                for sector in 0..sector_count {
                    let sector_offset = offset + SECTOR_INFO + sector as usize * SECTOR_INFO_SIZE;
                    let c = data[sector_offset];
                    let h = data[sector_offset + 1];
                    let r = data[sector_offset + 2];
                    let n = data[sector_offset + 3];
                    let st1 = data[sector_offset + 4];
                    let st2 = data[sector_offset + 5];
                    let data_length = if format == Format::Standard {
                        1 << (7 + n)
                    } else {
                        (data[sector_offset + 6] as u16) | ((data[sector_offset + 7] as u16) << 8)
                    };

                    sectors.push(SectorInfo {
                        track: c,
                        side: h,
                        id: r,
                        size: n,
                        st1,
                        st2,
                        data_length,
                        data: Vec::with_capacity(data_length as usize),
                    });
                }
                let mut sector_data_start =
                    (offset + SECTOR_INFO + sector_count as usize * SECTOR_INFO_SIZE + 0xFF)
                        / 0x100
                        * 0x100;
                for sector in sectors.iter_mut() {
                    if sector_data_start + sector.data_length as usize
                        > offset + track_size as usize
                    {
                        return Err(DiskError::Truncated);
                    }

                    sector.data.extend_from_slice(
                        &data[sector_data_start..(sector_data_start + sector.data_length as usize)],
                    );

                    sector_data_start += sector.data_length as usize;
                }

                tracks.push(Some(TrackInfo {
                    track_size,
                    track_number,
                    side_number,
                    data_rate,
                    recording_mode,
                    sector_size,
                    sector_count,
                    gap3,
                    filler,
                    sectors: sectors,
                }));

                offset += track_size as usize;
            }
        }

        Ok(Self {
            creator,
            format,
            track_count,
            side_count,
            tracks,
        })
    }

    fn format(&self) -> Format {
        self.format
    }

    fn creator(&self) -> &[u8] {
        let len = self
            .creator
            .iter()
            .position(|x| *x == 0)
            .unwrap_or(self.creator.len());
        &self.creator[..len]
    }

    fn track_count(&self) -> u8 {
        self.track_count
    }

    pub fn side_count(&self) -> u8 {
        self.side_count
    }

    fn track(&self, track: u8, side: u8) -> Option<&TrackInfo> {
        if track >= self.track_count || side >= self.side_count {
            None
        } else {
            self.tracks[self.side_count as usize * track as usize + side as usize].as_ref()
        }
    }

    fn track_size(&self, track: u8, side: u8) -> Option<u16> {
        self.track(track, side).map(|track| track.track_size)
    }

    pub fn sector_data(&self, track: u8, side: u8, sector_index: usize) -> Option<&[u8]> {
        self.track(track, side)
            .and_then(|track| track.sectors.get(sector_index))
            .map(|sector| sector.data.as_slice())
    }

    pub fn sector_data_by_id(&self, track: u8, side: u8, sector_id: u8) -> Option<&[u8]> {
        self.track(track, side)
            .and_then(|track| track.sectors.iter().find(|sector| sector.id == sector_id))
            .map(|sector| sector.data.as_slice())
    }

    pub fn sector_info_by_id(
        &self,
        track: u8,
        side: u8,
        sector_id: u8,
    ) -> Option<(u8, u8, u8, u8)> {
        self.track(track, side)
            .and_then(|t| t.sectors.iter().find(|s| s.id == sector_id))
            .map(|s| (s.track, s.side, s.id, s.size))
    }

    pub fn first_sector_info(&self, track: u8, side: u8) -> Option<(u8, u8, u8, u8)> {
        self.track(track, side)
            .and_then(|t| t.sectors.first())
            .map(|s| (s.track, s.side, s.id, s.size))
    }

    pub fn is_track_formatted(&self, track: u8, side: u8) -> bool {
        self.track(track, side).is_some()
    }

    fn track_mut(&mut self, track: u8, side: u8) -> Option<&mut TrackInfo> {
        if track >= self.track_count || side >= self.side_count {
            return None;
        }
        self.tracks[self.side_count as usize * track as usize + side as usize].as_mut()
    }

    pub fn write_sector_data(&mut self, track: u8, side: u8, sector_id: u8, data: &[u8]) -> bool {
        if let Some(ti) = self.track_mut(track, side) {
            for s in ti.sectors.iter_mut() {
                if s.id == sector_id {
                    let len = data.len().min(s.data.len());
                    s.data[..len].copy_from_slice(&data[..len]);
                    return true;
                }
            }
        }
        false
    }

    pub fn format_track(
        &mut self,
        track: u8,
        side: u8,
        sectors: &[(u8, u8, u8, u8)],
        n: u8,
        gap3: u8,
        filler: u8,
    ) {
        let sz = 1u16 << (n + 7);
        let sec_count = sectors.len() as u8;
        let track_size = 256 + sec_count as u16 * sz;
        let new_sectors: Vec<SectorInfo> = sectors
            .iter()
            .map(|&(c, h, r, n)| SectorInfo {
                track: c,
                side: h,
                id: r,
                size: n,
                st1: 0,
                st2: 0,
                data_length: sz,
                data: vec![filler; sz as usize],
            })
            .collect();
        let ti = TrackInfo {
            track_size,
            track_number: track,
            side_number: side,
            data_rate: DataRate::SdDd,
            recording_mode: RecordingMode::Mfm,
            sector_size: n,
            sector_count: sec_count,
            gap3,
            filler,
            sectors: new_sectors,
        };
        let idx = self.side_count as usize * track as usize + side as usize;
        while self.tracks.len() <= idx {
            self.tracks.push(None);
        }
        self.tracks[idx] = Some(ti);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    struct SSpec {
        c: u8,
        h: u8,
        r: u8,
        n: u8,
        st1: u8,
        st2: u8,
        dl: u16,
    }

    fn ss(c: u8, h: u8, r: u8, n: u8) -> SSpec {
        SSpec {
            c,
            h,
            r,
            n,
            st1: 0,
            st2: 0,
            dl: 0,
        }
    }

    fn ss_ext(c: u8, h: u8, r: u8, n: u8, dl: u16) -> SSpec {
        SSpec {
            c,
            h,
            r,
            n,
            st1: 0,
            st2: 0,
            dl,
        }
    }

    fn ss_st(c: u8, h: u8, r: u8, n: u8, st1: u8, st2: u8) -> SSpec {
        SSpec {
            c,
            h,
            r,
            n,
            st1,
            st2,
            dl: 0,
        }
    }

    fn standard_sectors(track: u8, side: u8, count: u8, first_id: u8, n: u8) -> Vec<SSpec> {
        (0..count)
            .map(|i| ss(track, side, first_id + i, n))
            .collect()
    }

    fn extended_sectors(track: u8, side: u8, count: u8, first_id: u8, n: u8) -> Vec<SSpec> {
        let sz = 1u16 << (n + 7);
        (0..count)
            .map(|i| ss_ext(track, side, first_id + i, n, sz))
            .collect()
    }

    fn make_dib_standard(creator: &[u8], tracks: u8, sides: u8, track_size: u16) -> Vec<u8> {
        let mut dib = vec![0u8; 256];
        dib[0..11].copy_from_slice(STANDARD_HEADER);
        let len = creator.len().min(14);
        dib[CREATOR..CREATOR + len].copy_from_slice(&creator[..len]);
        dib[TRACKS] = tracks;
        dib[SIDES] = sides;
        dib[TRACK_SIZE..TRACK_SIZE + 2].copy_from_slice(&track_size.to_le_bytes());
        dib
    }

    fn make_dib_extended(creator: &[u8], tracks: u8, sides: u8, track_msbs: &[u8]) -> Vec<u8> {
        let mut dib = vec![0u8; 256];
        dib[0..21].copy_from_slice(EXTENDED_HEADER);
        let len = creator.len().min(14);
        dib[CREATOR..CREATOR + len].copy_from_slice(&creator[..len]);
        dib[TRACKS] = tracks;
        dib[SIDES] = sides;
        for (i, &msb) in track_msbs.iter().enumerate() {
            if TRACK_SIZE_TABLE + i < 256 {
                dib[TRACK_SIZE_TABLE + i] = msb;
            }
        }
        dib
    }

    fn make_tib(track: u8, side: u8, sectors: &[SSpec], n: u8, gap3: u8, filler: u8) -> Vec<u8> {
        let required_size = SECTOR_INFO + sectors.len() * SECTOR_INFO_SIZE;
        let tib_size = if required_size <= 256 { 256 } else { 512 }; // Expand to 512 if overflowing

        let mut tib = vec![0u8; tib_size];
        tib[0..12].copy_from_slice(TRACK_HEADER);
        tib[TRACK_NUMBER] = track;
        tib[SIDE_NUMBER] = side;
        tib[DATA_RATE] = 1;
        tib[REC_MODE] = 2;
        tib[SECTOR_SIZE] = n;
        tib[SECTOR_COUNT] = sectors.len() as u8;
        tib[GAP3] = gap3;
        tib[FILLER] = filler;
        for (i, s) in sectors.iter().enumerate() {
            let off = SECTOR_INFO + i * SECTOR_INFO_SIZE;
            tib[off] = s.c;
            tib[off + 1] = s.h;
            tib[off + 2] = s.r;
            tib[off + 3] = s.n;
            tib[off + 4] = s.st1;
            tib[off + 5] = s.st2;
            tib[off + 6..off + 8].copy_from_slice(&s.dl.to_le_bytes());
        }
        tib
    }

    fn fill_sector_data(track: u8, side: u8, sector_idx: usize, size: usize) -> Vec<u8> {
        let byte = ((track as usize * 16 + side as usize * 4 + sector_idx + 1) & 0xFF) as u8;
        vec![byte; size]
    }

    fn make_standard_track(track: u8, side: u8, sectors: &[SSpec], n: u8) -> Vec<u8> {
        let sector_size = 1usize << (n + 7);
        let tib = make_tib(track, side, sectors, n, 0x4E, 0xE5);
        let mut data = tib;
        for i in 0..sectors.len() {
            data.extend_from_slice(&fill_sector_data(track, side, i, sector_size));
        }
        data
    }

    fn make_extended_track(track: u8, side: u8, sectors: &[SSpec], n: u8) -> Vec<u8> {
        let tib = make_tib(track, side, sectors, n, 0x4E, 0xE5);
        let mut data = tib;
        for i in 0..sectors.len() {
            let sz = sectors[i].dl as usize;
            data.extend_from_slice(&fill_sector_data(track, side, i, sz));
        }
        while data.len() % 256 != 0 {
            data.push(0);
        }
        data
    }

    fn build_standard_dsk(
        creator: &[u8],
        tracks: u8,
        sides: u8,
        n: u8,
        sectors_per_track: u8,
        first_id: u8,
    ) -> Vec<u8> {
        let sector_size = 1usize << (n + 7);
        let track_size = (256 + sectors_per_track as usize * sector_size) as u16;
        let mut disk = make_dib_standard(creator, tracks, sides, track_size);
        for t in 0..tracks {
            for s in 0..sides {
                let secs = standard_sectors(t, s, sectors_per_track, first_id, n);
                disk.extend_from_slice(&make_standard_track(t, s, &secs, n));
            }
        }
        disk
    }

    fn build_extended_dsk(
        creator: &[u8],
        tracks: u8,
        sides: u8,
        n: u8,
        sectors_per_track: u8,
        first_id: u8,
    ) -> Vec<u8> {
        let mut track_datas = Vec::new();
        let mut track_msbs = Vec::new();
        for t in 0..tracks {
            for s in 0..sides {
                let secs = extended_sectors(t, s, sectors_per_track, first_id, n);
                let td = make_extended_track(t, s, &secs, n);
                track_msbs.push((td.len() / 256) as u8);
                track_datas.push(td);
            }
        }
        let mut disk = make_dib_extended(creator, tracks, sides, &track_msbs);
        for td in &track_datas {
            disk.extend_from_slice(td);
        }
        disk
    }

    #[test]
    fn standard_dsk_header_detected() {
        let data = build_standard_dsk(b"CPC", 1, 1, 2, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.format(), Format::Standard);
    }

    #[test]
    fn extended_dsk_header_detected() {
        let data = build_extended_dsk(b"CPC", 1, 1, 2, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.format(), Format::Extended);
    }

    #[test]
    fn invalid_header_rejected() {
        let mut data = vec![0u8; 256];
        data[0..4].copy_from_slice(b"XXXX");
        assert_eq!(Disk::from_bytes(&data), Err(DiskError::InvalidHeader));
    }

    #[test]
    fn empty_input_rejected() {
        assert_eq!(Disk::from_bytes(&[]), Err(DiskError::Truncated));
    }

    #[test]
    fn truncated_dib_rejected() {
        let data = vec![0u8; 100];
        assert_eq!(Disk::from_bytes(&data), Err(DiskError::Truncated));
    }

    #[test]
    fn standard_dsk_creator_parsed() {
        let data = build_standard_dsk(b"Something", 1, 1, 2, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.creator(), b"Something");
    }

    #[test]
    fn extended_dsk_creator_parsed() {
        let data = build_extended_dsk(b"Rustrad", 1, 1, 2, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.creator(), b"Rustrad");
    }

    #[test]
    fn creator_trailing_nulls_trimmed() {
        let creator = b"ABCD\0\0\0\0\0\0";
        let data = build_standard_dsk(creator, 1, 1, 2, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.creator(), b"ABCD");
    }

    #[test]
    fn creator_exactly_14_chars() {
        let creator = b"12345678901234";
        let data = build_standard_dsk(creator, 1, 1, 2, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.creator(), b"12345678901234");
    }

    #[test]
    fn creator_empty_all_nulls() {
        let data = build_standard_dsk(b"", 1, 1, 2, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.creator(), b"");
    }

    #[test]
    fn standard_dsk_track_count() {
        let data = build_standard_dsk(b"C", 40, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track_count(), 40);
    }

    #[test]
    fn standard_dsk_side_count() {
        let data = build_standard_dsk(b"C", 40, 2, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.side_count(), 2);
    }

    #[test]
    fn disk_with_0_sides() {
        let data = build_standard_dsk(b"C", 40, 0, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data);
        assert_matches!(disk, Err(DiskError::InvalidHeader));
    }

    #[test]
    fn disk_with_3_sides() {
        let data = build_standard_dsk(b"C", 40, 3, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data);
        assert_matches!(disk, Err(DiskError::InvalidHeader));
    }

    #[test]
    fn extended_dsk_track_count() {
        let data = build_extended_dsk(b"C", 80, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track_count(), 80);
    }

    #[test]
    fn extended_dsk_side_count() {
        let data = build_extended_dsk(b"C", 80, 2, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.side_count(), 2);
    }

    #[test]
    fn standard_dsk_track_size_from_dib() {
        // 9 sectors of 512 bytes: 256 + 4608 = 4864
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track_size(0, 0), Some(4864));
    }

    #[test]
    fn extended_dsk_track_size_from_table() {
        let data = build_extended_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        // 9 sectors of 512: 256 + 4608 = 4864 = 19 * 256
        assert_eq!(disk.track_size(0, 0), Some(4864));
    }

    #[test]
    fn standard_dsk_all_tracks_same_size() {
        let data = build_standard_dsk(b"C", 3, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for t in 0..3 {
            assert_eq!(disk.track_size(t, 0), Some(4864));
        }
    }

    #[test]
    fn extended_dsk_variable_track_sizes() {
        // Build a custom extended DSK with different track sizes
        let t0_sectors = extended_sectors(0, 0, 9, 0xC1, 2); // 9 * 512 = 4608
        let t1_sectors = extended_sectors(1, 0, 5, 0xC1, 2); // 5 * 512 = 2560

        let t0_data = make_extended_track(0, 0, &t0_sectors, 2);
        let t1_data = make_extended_track(1, 0, &t1_sectors, 2);

        let track_msbs = vec![(t0_data.len() / 256) as u8, (t1_data.len() / 256) as u8];

        let mut disk = make_dib_extended(b"C", 2, 1, &track_msbs);
        disk.extend_from_slice(&t0_data);
        disk.extend_from_slice(&t1_data);

        let parsed = Disk::from_bytes(&disk).unwrap();
        assert_eq!(
            parsed.track_size(0, 0),
            Some((t0_data.len() / 256 * 256) as u16)
        );
        assert_eq!(
            parsed.track_size(1, 0),
            Some((t1_data.len() / 256 * 256) as u16)
        );
        assert_ne!(parsed.track_size(0, 0), parsed.track_size(1, 0));
    }

    #[test]
    fn track_number_correct() {
        let data = build_standard_dsk(b"C", 3, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track(0, 0).unwrap().track_number, 0);
        assert_eq!(disk.track(1, 0).unwrap().track_number, 1);
        assert_eq!(disk.track(2, 0).unwrap().track_number, 2);
    }

    #[test]
    fn side_number_correct() {
        let data = build_standard_dsk(b"C", 1, 2, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track(0, 0).unwrap().side_number, 0);
        assert_eq!(disk.track(0, 1).unwrap().side_number, 1);
    }

    #[test]
    fn data_rate_correct() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track(0, 0).unwrap().data_rate, DataRate::SdDd);
    }

    #[test]
    fn recording_mode_correct() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track(0, 0).unwrap().recording_mode, RecordingMode::Mfm);
    }

    #[test]
    fn sector_size_shift_correct() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track(0, 0).unwrap().sector_size, 2);
    }

    #[test]
    fn sector_count_correct() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track(0, 0).unwrap().sector_count, 9);
    }

    #[test]
    fn gap3_correct() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track(0, 0).unwrap().gap3, 0x4E);
    }

    #[test]
    fn filler_correct() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track(0, 0).unwrap().filler, 0xE5);
    }

    #[test]
    fn track_info_custom_gap3_and_filler() {
        let sectors = standard_sectors(0, 0, 1, 0x41, 2);
        let tib = make_tib(0, 0, &sectors, 2, 0x52, 0x00);
        let track_size = (256 + 512) as u16;
        let mut disk = make_dib_standard(b"C", 1, 1, track_size);
        disk.extend_from_slice(&tib);
        disk.extend_from_slice(&fill_sector_data(0, 0, 0, 512));

        let parsed = Disk::from_bytes(&disk).unwrap();
        let t = parsed.track(0, 0).unwrap();
        assert_eq!(t.gap3, 0x52);
        assert_eq!(t.filler, 0x00);
    }

    #[test]
    fn out_of_range_track_returns_none() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        assert!(disk.track(99, 0).is_none());
        assert!(disk.track(0, 99).is_none());
    }

    #[test]
    fn sector_chr_correct() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        let sectors = &disk.track(0, 0).unwrap().sectors;
        assert_eq!(sectors.len(), 9);
        for (i, s) in sectors.iter().enumerate() {
            assert_eq!(s.track, 0, "sector {} track", i);
            assert_eq!(s.side, 0, "sector {} side", i);
            assert_eq!(s.id, 0xC1 + i as u8, "sector {} id", i);
            assert_eq!(s.size, 2, "sector {} N", i);
        }
    }

    #[test]
    fn standard_dsk_st1_st2_zero() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for s in &disk.track(0, 0).unwrap().sectors {
            assert_eq!(s.st1, 0);
            assert_eq!(s.st2, 0);
        }
    }

    #[test]
    fn standard_dsk_data_length_is_2_pow_n_plus_7() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for s in &disk.track(0, 0).unwrap().sectors {
            // Standard DSK stores 0 in file; parser should set to 2^(N+7) = 512
            assert_eq!(s.data_length, 512);
        }
    }

    #[test]
    fn standard_dsk_n0_data_length_128() {
        let data = build_standard_dsk(b"C", 1, 1, 0, 1, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        let s = &disk.track(0, 0).unwrap().sectors[0];
        assert_eq!(s.size, 0);
        assert_eq!(s.data_length, 128);
    }

    #[test]
    fn extended_dsk_data_length_from_file() {
        let data = build_extended_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for s in &disk.track(0, 0).unwrap().sectors {
            assert_eq!(s.data_length, 512);
        }
    }

    #[test]
    fn extended_dsk_st1_st2_preserved() {
        let sectors = vec![
            ss_st(0, 0, 0xC1, 2, 0x80, 0x00),
            ss_st(0, 0, 0xC2, 2, 0x04, 0x40),
        ];
        let td = make_extended_track(0, 0, &sectors, 2);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors;
        assert_eq!(s[0].st1, 0x80);
        assert_eq!(s[0].st2, 0x00);
        assert_eq!(s[1].st1, 0x04);
        assert_eq!(s[1].st2, 0x40);
    }

    #[test]
    fn sector_data_by_index() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        let sd = disk.sector_data(0, 0, 0).unwrap();
        assert_eq!(sd.len(), 512);
    }

    #[test]
    fn sector_data_content_correct_standard() {
        let data = build_standard_dsk(b"C", 2, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for t in 0..2u8 {
            for i in 0..9usize {
                let sd = disk.sector_data(t, 0, i).unwrap();
                let expected = ((t as usize * 16 + i + 1) & 0xFF) as u8;
                assert!(
                    sd.iter().all(|&b| b == expected),
                    "track {} sector {} expected all {:02X}",
                    t,
                    i,
                    expected
                );
            }
        }
    }

    #[test]
    fn sector_data_content_correct_extended() {
        let data = build_extended_dsk(b"C", 2, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for t in 0..2u8 {
            for i in 0..9usize {
                let sd = disk.sector_data(t, 0, i).unwrap();
                let expected = ((t as usize * 16 + i + 1) & 0xFF) as u8;
                assert!(
                    sd.iter().all(|&b| b == expected),
                    "track {} sector {} expected all {:02X}",
                    t,
                    i,
                    expected
                );
            }
        }
    }

    #[test]
    fn sector_data_different_per_sector() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 3, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        let s0 = disk.sector_data(0, 0, 0).unwrap();
        let s1 = disk.sector_data(0, 0, 1).unwrap();
        let s2 = disk.sector_data(0, 0, 2).unwrap();
        assert_ne!(s0[0], s1[0]);
        assert_ne!(s1[0], s2[0]);
        assert_ne!(s0[0], s2[0]);
    }

    #[test]
    fn sector_data_by_id() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        let sd = disk.sector_data_by_id(0, 0, 0xC5).unwrap();
        assert_eq!(sd.len(), 512);
        // Sector 0xC5 is index 4
        let expected = ((0 * 16 + 4 + 1) & 0xFF) as u8;
        assert_eq!(sd[0], expected);
    }

    #[test]
    fn sector_data_by_id_not_found() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert!(disk.sector_data_by_id(0, 0, 0xFF).is_none());
    }

    #[test]
    fn sector_data_out_of_range_index() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert!(disk.sector_data(0, 0, 9).is_none());
        assert!(disk.sector_data(0, 0, 99).is_none());
    }

    #[test]
    fn sector_data_different_n_values() {
        // N=1 -> 256 bytes per sector
        let data = build_standard_dsk(b"C", 1, 1, 1, 4, 0x41);
        let disk = Disk::from_bytes(&data).unwrap();
        let s = &disk.track(0, 0).unwrap().sectors[0];
        assert_eq!(s.size, 1);
        assert_eq!(s.data_length, 256);
        assert_eq!(disk.sector_data(0, 0, 0).unwrap().len(), 256);
    }

    #[test]
    fn non_sequential_sector_ids() {
        let sectors = vec![ss(0, 0, 0xC1, 2), ss(0, 0, 0xC5, 2), ss(0, 0, 0xC3, 2)];
        let td = make_standard_track(0, 0, &sectors, 2);
        let track_size = (256 + 3 * 512) as u16;
        let mut disk = make_dib_standard(b"C", 1, 1, track_size);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors;
        assert_eq!(s[0].id, 0xC1);
        assert_eq!(s[1].id, 0xC5);
        assert_eq!(s[2].id, 0xC3);

        // sector_data_by_id should find the right one
        let sd = parsed.sector_data_by_id(0, 0, 0xC5).unwrap();
        assert_eq!(sd.len(), 512);
    }

    #[test]
    fn standard_dsk_multi_track_single_side() {
        let data = build_standard_dsk(b"C", 5, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for t in 0..5 {
            assert!(disk.track(t, 0).is_some());
            assert_eq!(disk.track(t, 0).unwrap().sector_count, 9);
        }
    }

    #[test]
    fn standard_dsk_single_track_multi_side() {
        let data = build_standard_dsk(b"C", 1, 2, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert!(disk.track(0, 0).is_some());
        assert!(disk.track(0, 1).is_some());
    }

    #[test]
    fn standard_dsk_multi_track_multi_side() {
        let data = build_standard_dsk(b"C", 3, 2, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for t in 0..3 {
            for s in 0..2 {
                assert!(disk.track(t, s).is_some(), "track {} side {} missing", t, s);
                assert_eq!(disk.track(t, s).unwrap().track_number, t);
                assert_eq!(disk.track(t, s).unwrap().side_number, s);
            }
        }
    }

    #[test]
    fn extended_dsk_multi_track_multi_side() {
        let data = build_extended_dsk(b"C", 3, 2, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        for t in 0..3 {
            for s in 0..2 {
                assert!(disk.track(t, s).is_some(), "track {} side {} missing", t, s);
            }
        }
    }

    #[test]
    fn track_ordering_standard() {
        // Verify tracks are stored T0S0, T0S1, T1S0, T1S1, ...
        let data = build_standard_dsk(b"C", 2, 2, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        // Each track has unique data based on track number
        let t0s0 = disk.sector_data(0, 0, 0).unwrap();
        let t0s1 = disk.sector_data(0, 1, 0).unwrap();
        let t1s0 = disk.sector_data(1, 0, 0).unwrap();
        let t1s1 = disk.sector_data(1, 1, 0).unwrap();
        assert_ne!(t0s0[0], t0s1[0]);
        assert_ne!(t0s0[0], t1s0[0]);
        assert_ne!(t1s0[0], t1s1[0]);
    }

    #[test]
    fn track_offset_standard_calculation() {
        // 2 tracks, 1 side, 9 sectors of 512: track_size = 4864
        let data = build_standard_dsk(b"C", 2, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        // Track 0 data at offset 256, Track 1 at 256 + 4864 = 5120
        // Verify by checking track numbers in TIB
        assert_eq!(disk.track(0, 0).unwrap().track_number, 0);
        assert_eq!(disk.track(1, 0).unwrap().track_number, 1);
    }

    #[test]
    fn track_offset_extended_cumulative() {
        // Build extended DSK with different track sizes
        let t0_secs = extended_sectors(0, 0, 9, 0xC1, 2);
        let t1_secs = extended_sectors(1, 0, 5, 0xC1, 2);
        let t0_data = make_extended_track(0, 0, &t0_secs, 2);
        let t1_data = make_extended_track(1, 0, &t1_secs, 2);
        let msbs = vec![(t0_data.len() / 256) as u8, (t1_data.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 2, 1, &msbs);
        disk.extend_from_slice(&t0_data);
        disk.extend_from_slice(&t1_data);

        let parsed = Disk::from_bytes(&disk).unwrap();
        // Verify the correct tracks are at the correct offsets
        assert_eq!(parsed.track(0, 0).unwrap().track_number, 0);
        assert_eq!(parsed.track(0, 0).unwrap().sector_count, 9);
        assert_eq!(parsed.track(1, 0).unwrap().track_number, 1);
        assert_eq!(parsed.track(1, 0).unwrap().sector_count, 5);
    }

    #[test]
    fn extended_dsk_unformatted_track() {
        // Track 0 formatted, Track 1 unformatted (MSB = 0)
        let t0_secs = extended_sectors(0, 0, 9, 0xC1, 2);
        let t0_data = make_extended_track(0, 0, &t0_secs, 2);
        let msbs = vec![(t0_data.len() / 256) as u8, 0];
        let mut disk = make_dib_extended(b"C", 2, 1, &msbs);
        disk.extend_from_slice(&t0_data);

        let parsed = Disk::from_bytes(&disk).unwrap();
        assert!(parsed.is_track_formatted(0, 0));
        assert!(!parsed.is_track_formatted(1, 0));
    }

    #[test]
    fn is_track_formatted_true_for_standard() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert!(disk.is_track_formatted(0, 0));
    }

    #[test]
    fn is_track_formatted_false_for_out_of_range() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert!(!disk.is_track_formatted(99, 0));
        assert!(!disk.is_track_formatted(0, 99));
    }

    #[test]
    fn unformatted_track_returns_none() {
        let t0_secs = extended_sectors(0, 0, 9, 0xC1, 2);
        let t0_data = make_extended_track(0, 0, &t0_secs, 2);
        let msbs = vec![(t0_data.len() / 256) as u8, 0];
        let mut disk = make_dib_extended(b"C", 2, 1, &msbs);
        disk.extend_from_slice(&t0_data);

        let parsed = Disk::from_bytes(&disk).unwrap();
        assert!(parsed.track(1, 0).is_none());
        assert!(parsed.sector_data(1, 0, 0).is_none());
    }

    #[test]
    fn all_unformatted_extended_dsk() {
        let disk = make_dib_extended(b"C", 3, 1, &[0, 0, 0]);
        let parsed = Disk::from_bytes(&disk).unwrap();
        for t in 0..3 {
            assert!(!parsed.is_track_formatted(t, 0));
            assert!(parsed.track(t, 0).is_none());
        }
    }

    #[test]
    fn short_sector_data_length() {
        // N=2 (512 bytes) but data_length = 128
        let sectors = vec![ss_ext(0, 0, 0xC1, 2, 128)];
        let td = make_extended_track(0, 0, &sectors, 2);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors[0];
        assert_eq!(s.size, 2);
        assert_eq!(s.data_length, 128);
        assert_eq!(parsed.sector_data(0, 0, 0).unwrap().len(), 128);
    }

    #[test]
    fn large_sector_n6_16k() {
        // N=6 -> 2^13 = 8192 bytes
        let sectors = vec![ss_ext(0, 0, 0xC1, 6, 8192)];
        let td = make_extended_track(0, 0, &sectors, 6);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors[0];
        assert_eq!(s.size, 6);
        assert_eq!(s.data_length, 8192);
        assert_eq!(parsed.sector_data(0, 0, 0).unwrap().len(), 8192);
    }

    #[test]
    fn large_sector_n8_32k() {
        // N=8 -> 2^15 = 32768 bytes
        let sectors = vec![ss_ext(0, 0, 0xC1, 8, 32768)];
        let td = make_extended_track(0, 0, &sectors, 8);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors[0];
        assert_eq!(s.size, 8);
        assert_eq!(s.data_length, 32768);
        assert_eq!(parsed.sector_data(0, 0, 0).unwrap().len(), 32768);
    }

    #[test]
    fn weak_sector_two_copies() {
        // N=2 (512 bytes), data_length = 1024 (2 copies)
        let sectors = vec![ss_ext(0, 0, 0xC1, 2, 1024)];
        let td = make_extended_track(0, 0, &sectors, 2);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors[0];
        assert_eq!(s.data_length, 1024);
        assert_eq!(s.data_length % 512, 0);
        assert!(s.data_length > 512);

        let sd = parsed.sector_data(0, 0, 0).unwrap();
        assert_eq!(sd.len(), 1024);
    }

    #[test]
    fn weak_sector_three_copies() {
        // N=2 (512 bytes), data_length = 1536 (3 copies)
        let sectors = vec![ss_ext(0, 0, 0xC1, 2, 1536)];
        let td = make_extended_track(0, 0, &sectors, 2);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors[0];
        assert_eq!(s.data_length, 1536);
        assert_eq!(s.data_length % 512, 0);
        assert!(s.data_length > 512);

        let sd = parsed.sector_data(0, 0, 0).unwrap();
        assert_eq!(sd.len(), 1536);
    }

    #[test]
    fn gap3_data_sector() {
        // N=2 (512 bytes), data_length = 612 (512 + 100 extra for gap3)
        let sectors = vec![ss_ext(0, 0, 0xC1, 2, 612)];
        let td = make_extended_track(0, 0, &sectors, 2);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors[0];
        assert_eq!(s.data_length, 612);
        assert!(s.data_length > 512);
        assert_ne!(s.data_length % 512, 0);

        let sd = parsed.sector_data(0, 0, 0).unwrap();
        assert_eq!(sd.len(), 612);
    }

    #[test]
    fn mixed_sector_types_on_same_track() {
        let sectors = vec![
            ss_ext(0, 0, 0xC1, 2, 512),  // normal
            ss_ext(0, 0, 0xC2, 2, 128),  // short
            ss_ext(0, 0, 0xC3, 2, 1024), // weak (2 copies)
        ];
        let td = make_extended_track(0, 0, &sectors, 2);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors;
        assert_eq!(s[0].data_length, 512);
        assert_eq!(s[1].data_length, 128);
        assert_eq!(s[2].data_length, 1024);

        assert_eq!(parsed.sector_data(0, 0, 0).unwrap().len(), 512);
        assert_eq!(parsed.sector_data(0, 0, 1).unwrap().len(), 128);
        assert_eq!(parsed.sector_data(0, 0, 2).unwrap().len(), 1024);
    }

    #[test]
    fn track_with_zero_sectors() {
        let sectors: Vec<SSpec> = vec![];
        let td = make_tib(0, 0, &sectors, 2, 0x4E, 0xE5);
        let track_size = 256u16;
        let mut disk = make_dib_standard(b"C", 1, 1, track_size);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let t = parsed.track(0, 0).unwrap();
        assert_eq!(t.sector_count, 0);
        assert!(t.sectors.is_empty());
        assert!(parsed.sector_data(0, 0, 0).is_none());
    }

    #[test]
    fn track_with_max_29_sectors() {
        // (256 - 24) / 8 = 29 sectors fit in a 256-byte TIB
        let sectors: Vec<SSpec> = (0..29).map(|i| ss(0, 0, 0x41 + i, 2)).collect();
        let td = make_standard_track(0, 0, &sectors, 2);
        let track_size = (256 + 29 * 512) as u16;
        let mut disk = make_dib_standard(b"C", 1, 1, track_size);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let t = parsed.track(0, 0).unwrap();
        assert_eq!(t.sector_count, 29);
        assert_eq!(t.sectors.len(), 29);
        for i in 0..29 {
            assert_eq!(t.sectors[i].id, 0x41 + i as u8);
            assert!(parsed.sector_data(0, 0, i).is_some());
        }
    }

    #[test]
    fn zero_track_count_extended_dsk() {
        let disk = make_dib_extended(b"C", 0, 1, &[]);
        let parsed = Disk::from_bytes(&disk).unwrap();
        assert_eq!(parsed.track_count(), 0);
    }

    #[test]
    fn different_n_per_sector() {
        let sectors = vec![
            ss_ext(0, 0, 0xC1, 2, 512),
            ss_ext(0, 0, 0xC2, 1, 256),
            ss_ext(0, 0, 0xC3, 0, 128),
        ];
        let td = make_extended_track(0, 0, &sectors, 2);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let s = &parsed.track(0, 0).unwrap().sectors;
        assert_eq!(s[0].size, 2);
        assert_eq!(s[0].data_length, 512);
        assert_eq!(s[1].size, 1);
        assert_eq!(s[1].data_length, 256);
        assert_eq!(s[2].size, 0);
        assert_eq!(s[2].data_length, 128);
    }

    #[test]
    fn truncated_track_data_standard() {
        let track_size = (256 + 9 * 512) as u16;
        let mut disk = make_dib_standard(b"C", 1, 1, track_size);
        let tib = make_tib(0, 0, &standard_sectors(0, 0, 9, 0xC1, 2), 2, 0x4E, 0xE5);
        disk.extend_from_slice(&tib);
        // Only add 100 bytes of sector data instead of 4608
        disk.extend_from_slice(&vec![0u8; 100]);

        assert_eq!(Disk::from_bytes(&disk), Err(DiskError::Truncated));
    }

    #[test]
    fn truncated_sector_data_extended() {
        let sectors = extended_sectors(0, 0, 9, 0xC1, 2);
        let td = make_extended_track(0, 0, &sectors, 2);
        let msbs = vec![(td.len() / 256) as u8];
        let mut disk = make_dib_extended(b"C", 1, 1, &msbs);
        // Truncate the track data
        disk.extend_from_slice(&td[..td.len() - 100]);

        let result = Disk::from_bytes(&disk);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_track_info_header() {
        let track_size = (256 + 512) as u16;
        let mut disk = make_dib_standard(b"C", 1, 1, track_size);
        // Create a TIB with wrong header
        let mut tib = vec![0u8; 256];
        tib[0..12].copy_from_slice(b"BAD-INFO\r\n\0\0");
        tib[16] = 0;
        tib[17] = 0;
        tib[20] = 2;
        tib[21] = 1;
        tib[24] = 0;
        tib[25] = 0;
        tib[26] = 0x41;
        tib[27] = 2;
        disk.extend_from_slice(&tib);
        disk.extend_from_slice(&fill_sector_data(0, 0, 0, 512));

        assert_eq!(Disk::from_bytes(&disk), Err(DiskError::InvalidTrackInfo));
    }

    #[test]
    fn truncated_track_info_block() {
        let track_size = (256 + 512) as u16;
        let mut disk = make_dib_standard(b"C", 1, 1, track_size);
        // Only 100 bytes of TIB instead of 256
        disk.extend_from_slice(&vec![0u8; 100]);

        assert_eq!(Disk::from_bytes(&disk), Err(DiskError::Truncated));
    }

    #[test]
    fn file_longer_than_expected_ok() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 1, 0x41);
        let mut extra = data.clone();
        extra.extend_from_slice(&vec![0xFF; 100]);
        // Should still parse OK (extra data ignored)
        let disk = Disk::from_bytes(&extra).unwrap();
        assert_eq!(disk.track_count(), 1);
    }

    #[test]
    fn standard_dsk_data_format_40_tracks() {
        // CPC DATA format: 40 tracks, 1 side, 9 sectors, IDs 0xC1-0xC9, 512 bytes
        let data = build_standard_dsk(b"Rustrad", 40, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track_count(), 40);
        assert_eq!(disk.side_count(), 1);
        assert_eq!(disk.format(), Format::Standard);

        for t in 0..40 {
            let tr = disk.track(t, 0).unwrap();
            assert_eq!(tr.sector_count, 9);
            for (i, s) in tr.sectors.iter().enumerate() {
                assert_eq!(s.id, 0xC1 + i as u8);
                assert_eq!(s.size, 2);
                assert_eq!(s.data_length, 512);
            }
        }

        // Total file size: 256 + 40 * 4864 = 256 + 194560 = 194816
        assert_eq!(data.len(), 256 + 40 * 4864);
    }

    #[test]
    fn extended_dsk_80_tracks_2_sides() {
        let data = build_extended_dsk(b"Rustrad", 80, 2, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        assert_eq!(disk.track_count(), 80);
        assert_eq!(disk.side_count(), 2);
        assert_eq!(disk.format(), Format::Extended);

        for t in 0..80 {
            for s in 0..2 {
                let tr = disk.track(t, s).unwrap();
                assert_eq!(tr.sector_count, 9);
                assert_eq!(tr.track_number, t);
                assert_eq!(tr.side_number, s);
            }
        }
    }

    #[test]
    fn standard_dsk_ibm_format() {
        // IBM format: 8 sectors, IDs 0x01-0x08
        let data = build_standard_dsk(b"Rustrad", 1, 1, 2, 8, 0x01);
        let disk = Disk::from_bytes(&data).unwrap();
        let tr = disk.track(0, 0).unwrap();
        assert_eq!(tr.sector_count, 8);
        for (i, s) in tr.sectors.iter().enumerate() {
            assert_eq!(s.id, 0x01 + i as u8);
        }
    }

    #[test]
    fn extended_dsk_with_unformatted_tracks_interleaved() {
        // 3 tracks: T0 formatted, T1 unformatted, T2 formatted
        let t0 = make_extended_track(0, 0, &extended_sectors(0, 0, 9, 0xC1, 2), 2);
        let t2 = make_extended_track(2, 0, &extended_sectors(2, 0, 9, 0xC1, 2), 2);
        let msbs = vec![
            (t0.len() / 256) as u8,
            0, // unformatted
            (t2.len() / 256) as u8,
        ];
        let mut disk = make_dib_extended(b"C", 3, 1, &msbs);
        disk.extend_from_slice(&t0);
        disk.extend_from_slice(&t2);

        let parsed = Disk::from_bytes(&disk).unwrap();
        assert!(parsed.is_track_formatted(0, 0));
        assert!(!parsed.is_track_formatted(1, 0));
        assert!(parsed.is_track_formatted(2, 0));

        assert_eq!(parsed.track(0, 0).unwrap().track_number, 0);
        assert_eq!(parsed.track(2, 0).unwrap().track_number, 2);
        assert_eq!(parsed.track(2, 0).unwrap().sector_count, 9);
    }

    #[test]
    fn sector_data_consistent_across_calls() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        let s1 = disk.sector_data(0, 0, 0).unwrap();
        let s2 = disk.sector_data(0, 0, 0).unwrap();
        assert_eq!(s1.len(), s2.len());
        assert_eq!(s1, s2);
    }

    #[test]
    fn track_info_sectors_vec_matches_count() {
        let data = build_standard_dsk(b"C", 1, 1, 2, 9, 0xC1);
        let disk = Disk::from_bytes(&data).unwrap();
        let t = disk.track(0, 0).unwrap();
        assert_eq!(t.sectors.len(), t.sector_count as usize);
    }

    #[test]
    fn extended_dsk_variable_track_sizes_multi_side() {
        let t0s0_secs = extended_sectors(0, 0, 9, 0xC1, 2);
        let t0s1_secs = extended_sectors(0, 1, 5, 0xC1, 2);
        let t1s0_secs = extended_sectors(1, 0, 5, 0xC1, 2);
        let t1s1_secs = extended_sectors(1, 1, 9, 0xC1, 2);

        let t0s0_data = make_extended_track(0, 0, &t0s0_secs, 2);
        let t0s1_data = make_extended_track(0, 1, &t0s1_secs, 2);
        let t1s0_data = make_extended_track(1, 0, &t1s0_secs, 2);
        let t1s1_data = make_extended_track(1, 1, &t1s1_secs, 2);

        let track_msbs = vec![
            (t0s0_data.len() / 256) as u8,
            (t0s1_data.len() / 256) as u8,
            (t1s0_data.len() / 256) as u8,
            (t1s1_data.len() / 256) as u8,
        ];

        let mut disk = make_dib_extended(b"C", 2, 2, &track_msbs);
        disk.extend_from_slice(&t0s0_data);
        disk.extend_from_slice(&t0s1_data);
        disk.extend_from_slice(&t1s0_data);
        disk.extend_from_slice(&t1s1_data);

        let parsed = Disk::from_bytes(&disk).unwrap();

        // Check that tracks mapped to the correct sizes from the table
        assert_eq!(
            parsed.track_size(0, 0),
            Some((t0s0_data.len() / 256 * 256) as u16)
        );
        assert_eq!(
            parsed.track_size(0, 1),
            Some((t0s1_data.len() / 256 * 256) as u16)
        );
        assert_eq!(
            parsed.track_size(1, 0),
            Some((t1s0_data.len() / 256 * 256) as u16)
        );
        assert_eq!(
            parsed.track_size(1, 1),
            Some((t1s1_data.len() / 256 * 256) as u16)
        );

        // Verify the sector counts align with the correctly parsed tracks
        assert_eq!(parsed.track(0, 0).unwrap().sector_count, 9);
        assert_eq!(parsed.track(0, 1).unwrap().sector_count, 5);
        assert_eq!(parsed.track(1, 0).unwrap().sector_count, 5);
        assert_eq!(parsed.track(1, 1).unwrap().sector_count, 9);
    }

    #[test]
    fn track_with_30_sectors_tib_overflow() {
        // 30 sectors * 8 bytes = 240 bytes for SIL.
        // SIL starts at offset 0x18 (24). 24 + 240 = 264 bytes.
        // This exceeds 256, forcing the TIB to pad out to 512 bytes before data starts.
        let sectors: Vec<SSpec> = (0..30).map(|i| ss(0, 0, 0x41 + i, 2)).collect();
        let td = make_standard_track(0, 0, &sectors, 2);
        let track_size = (512 + 30 * 512) as u16; // 512 for padded TIB + 15360 for data
        let mut disk = make_dib_standard(b"C", 1, 1, track_size);
        disk.extend_from_slice(&td);

        let parsed = Disk::from_bytes(&disk).unwrap();
        let t = parsed.track(0, 0).unwrap();
        assert_eq!(t.sector_count, 30);
        assert_eq!(t.sectors.len(), 30);

        // Validate that sector data is correctly sliced despite the overflow
        for i in 0..30usize {
            assert_eq!(t.sectors[i].id, 0x41 + i as u8);
            let data = parsed.sector_data(0, 0, i).unwrap();
            assert_eq!(data.len(), 512);
            let expected_byte = ((0 * 16 + 0 * 4 + i + 1) & 0xFF) as u8;
            assert_eq!(data[0], expected_byte);
        }
    }
}
