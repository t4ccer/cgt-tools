use num_derive::FromPrimitive;

use crate::dyadic_rational_number::DyadicRationalNumber;

// Cache constants
const STD_OPTIONS_RECORD: u32 = 0x00000000;
const EXT_OPTIONS_RECORD: u32 = 0x80000000;
const STD_NUS_RECORD: u32 = 0x40000000;
const EXT_NUS_RECORD: u32 = 0xc0000000;
const RECORD_TYPE_MASK: u32 = 0xc0000000;
const EXT_RECORD_MASK: u32 = 0x80000000;
const NUS_RECORD_MASK: u32 = 0x40000000;

// Standard options record descriptor (still 1 bit free):
const IS_NUS_MASK: u32 = 0x20000000;
const IS_NON_UPTIMAL_MASK: u32 = 0x10000000;
const NUM_LO_MASK: u32 = 0x0fffc000;
const NUM_LO_SHIFT: u32 = 14;
const NUM_RO_MASK: u32 = 0x00003fff;

// Standard Nus record descriptor:
// xxxxxxxx xxxxxxxx xxxxxxxx xxxxxxxx
//   |___||_____________||___________|
//     |          |            |
//  Denom.   Up multiple     Nimber
const DENOMINATOR_MASK: u32 = 0x3e000000;
const DENOMINATOR_SHIFT: u32 = 25;
const UP_MULTIPLE_MASK: u32 = 0x01fff000;
const UP_MULTIPLE_LEFTSHIFT: u32 = 7;
const UP_MULTIPLE_RIGHTSHIFT: u32 = 19;
const NIMBER_MASK: u32 = 0x00000fff;
const EXT_DENOMINATOR_MASK: u32 = !RECORD_TYPE_MASK;

const SECTOR_BITS: u32 = 18;
const SECTOR_SIZE: usize = 1 << SECTOR_BITS;
const SECTOR_MASK: usize = SECTOR_SIZE - 1;
const DEFAULT_INDEX_CAPACITY: usize = 1 << 16; // 256 KB (64K entries);
const DEFAULT_INDEX_MASK: usize = DEFAULT_INDEX_CAPACITY - 1;
const DEFAULT_SECTOR_SLOTS: usize = 16;
const UNUSED_BUCKET: u32 = -1i32 as u32;

const DEFAULT_OP_TABLE_SIZE: usize = 1 << 18;
const DEFAULT_OP_TABLE_MASK: usize = DEFAULT_OP_TABLE_SIZE - 1;

#[derive(Debug, FromPrimitive, Clone, Copy)]
enum Operation {
    None = 0,
    Sum = 1,
    Negative = 2,
    Birthday = 3,
    AtomicWeight = 4,
    NortonMultiply = 5,
    ConwayMultiply = 6,
    OrdinalSum = 7,
}

struct Hasher {}

impl Hasher {
    fn hash(inp: i32) -> i32 {
        let h: u32 = inp as u32;
        let (mut h, _) = h.overflowing_add(!(h << 9));
        h ^= h >> 14;
        let (mut h, _) = h.overflowing_add(h << 4);
        h ^= h >> 10;
        h as i32
    }
}

#[test]
fn hash_works() {
    assert_eq!(Hasher::hash(0), -8130816);
    assert_eq!(Hasher::hash(12), -8226735);
    assert_eq!(Hasher::hash(-42), 364656);
    assert_eq!(Hasher::hash(1337), -10294144);
    assert_eq!(Hasher::hash(482364747), 1588849805);
}

#[derive(Debug)]
struct Nus {
    number: DyadicRationalNumber,
    up_multiple: i32,
    nimber: i32,
}

impl Nus {
    fn is_small_nus(&self) -> bool {
        // TODO: r.numerator().isSmallInteger()
        self.nimber < 4096 && self.up_multiple >= -4096 && self.up_multiple < 4096
    }

    fn to_small(&self) -> Option<SmallNus> {
        if self.is_small_nus() {
            Some(SmallNus {
                numerator: self.number.numerator(),
                den_exp: self.number.denominator_exponent(),
                up_multiple: self.up_multiple,
                nimber: self.nimber,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct SmallNus {
    numerator: i32,
    den_exp: i32,
    up_multiple: i32,
    nimber: i32,
}

impl SmallNus {
    pub fn hash(&self) -> i32 {
        Self::hash_desc(self.descriptor() as i32, self.numerator)
    }

    fn hash_desc(descriptor: i32, numerator: i32) -> i32 {
        Hasher::hash(descriptor ^ numerator)
    }

    pub fn descriptor(&self) -> u32 {
        // TODO: Make sure that type coersions are sound
        let res = STD_NUS_RECORD
            | ((self.den_exp as u32) << DENOMINATOR_SHIFT)
            | (((self.up_multiple << UP_MULTIPLE_RIGHTSHIFT) as u32) >> UP_MULTIPLE_LEFTSHIFT)
            | self.nimber as u32;
        res
    }
}

/// Like arraycopy
fn copy_slice<T>(src: &[T], src_start: usize, dst: &mut [T], dst_start: usize, len: usize)
where
    T: Clone,
{
    let src_end = src_start + len;
    let es = &src[src_start..=src_end];
    for (idx, e) in es.iter().cloned().enumerate() {
        dst[dst_start + idx] = e;
    }
}

#[derive(Debug)]
pub struct GameStorage {
    index_capacity: usize,
    index_mask: usize,
    index: Vec<u32>,
    data: Vec<Vec<u32>>,
    next_offset: u32,
    next_sector: i32,
    total_records: usize,
    // Operations
    op_table_op: Vec<Operation>,
    op_table_g: Vec<u32>,
    op_table_h: Vec<u32>,
    op_table_result: Vec<i32>,
    // known IDs
    pub zero_id: u32,
    pub star_id: u32,
}

impl GameStorage {
    fn write_to_index(&mut self, hashcode: i32, value: u32) {
        if self.total_records > self.index_capacity {
            self.total_records = 0;
            // self.grow_index_and_rehash();
            todo!("resize");
        }

        let bucket = (hashcode as usize) & self.index_mask;
        if self.index[bucket] == UNUSED_BUCKET {
            self.index[bucket] = value;
        } else {
            let mut offset_at = self.index[bucket] as usize;
            while self.data[offset_at >> SECTOR_BITS][offset_at & SECTOR_MASK] != UNUSED_BUCKET {
                offset_at = self.data[offset_at >> SECTOR_BITS][offset_at & SECTOR_MASK] as usize;
            }
            self.data[offset_at >> SECTOR_BITS][offset_at & SECTOR_MASK] = value;
        }
    }

    fn hash_options(
        num_lo: u32,
        lo_array: &[u32],
        lo_offset: u32,
        num_ro: u32,
        ro_array: &[u32],
        ro_offset: u32,
    ) -> i32 {
        let mut res: i32 = 1;

        for i in 0..num_lo {
            let (hashcode, _) = res.overflowing_mul(32);
            let (hashcode, _) = hashcode.overflowing_add(lo_array[(lo_offset + i) as usize] as i32);
            res = hashcode;
        }
        for i in 0..num_ro {
            let (hashcode, _) = res.overflowing_mul(32);
            let (hashcode, _) = hashcode.overflowing_add(ro_array[(ro_offset + i) as usize] as i32);
            res = hashcode;
        }

        Hasher::hash(res)
    }

    fn lookup_nus_record(&self, nus: &Nus) -> u32 {
        match nus.to_small() {
            Some(small_nus) => self.lookup_small_nus_record(&small_nus),
            None => todo!(),
        }
    }

    fn lookup_small_nus_record(&self, small_nus: &SmallNus) -> u32 {
        let mut offset_at = self.index[(small_nus.hash() as usize) & self.index_mask];
        if offset_at == UNUSED_BUCKET {
            return UNUSED_BUCKET;
        }

        let descriptor = small_nus.descriptor();
        while offset_at != UNUSED_BUCKET {
            let sector = &self.data[(offset_at >> SECTOR_BITS) as usize];
            let sector_offset = (offset_at as usize) & SECTOR_MASK;
            if sector[sector_offset + 1] == descriptor
                && sector[sector_offset + 2] == (small_nus.numerator as u32)
            {
                return offset_at + 3;
            }
            offset_at = sector[sector_offset];
        }

        return UNUSED_BUCKET;
    }

    fn construct_rational(number: DyadicRationalNumber) -> u32 {
        todo!()
    }

    fn ensure_sector_space(&mut self, needed_slots: u32) {
        if (self.next_offset >> SECTOR_BITS) >= (self.next_sector as u32)
            || SECTOR_SIZE - ((self.next_offset as usize) & SECTOR_MASK) < (needed_slots as usize)
        {
            if (self.next_sector as usize) >= self.data.len() {
                self.data
                    .extend_reserve((self.next_sector as usize) - self.data.len());
            }

            self.data[self.next_sector as usize] = vec![0; SECTOR_SIZE];
            self.next_offset = (self.next_sector as u32) << SECTOR_BITS;
            self.next_sector += 1;
        }
    }

    fn write_nus_record_execpt_options(&mut self, nus: &Nus, num_lo: u32, num_ro: u32) -> u32 {
        if num_lo >= 16384 || num_ro >= 16384 {
            panic!("Too many options");
        }

        match nus.to_small() {
            Some(small_nus) => {
                self.ensure_sector_space(5 + num_lo + num_ro);
                let sector = unsafe {
                    self.data
                        .get_unchecked_mut((self.next_offset >> SECTOR_BITS) as usize)
                };
                let sector_offset = (self.next_offset as usize) & SECTOR_MASK;

                sector[sector_offset] = UNUSED_BUCKET;
                sector[sector_offset + 1] = small_nus.descriptor();
                sector[sector_offset + 2] = small_nus.numerator as u32;
                sector[sector_offset + 3] = UNUSED_BUCKET;
                sector[sector_offset + 4] =
                    STD_OPTIONS_RECORD | IS_NUS_MASK | (num_lo << NUM_LO_SHIFT) | num_ro;

                self.write_to_index(small_nus.hash(), self.next_offset);
                let options_offset = self.next_offset + 3;

                self.next_offset += 5 + num_lo + num_ro;

                options_offset
            }
            None => todo!(),
        }
    }

    fn construct_nimber(&mut self, number: DyadicRationalNumber, nimber: i32) -> u32 {
        let mut last_defined: i32 = nimber;
        let mut offset_at: u32 = UNUSED_BUCKET;
        loop {
            let nus = Nus {
                number,
                up_multiple: 0,
                nimber: last_defined,
            };
            offset_at = self.lookup_nus_record(&nus);
            if offset_at != UNUSED_BUCKET || last_defined <= 0 {
                break;
            }
            last_defined -= 1;
        }

        if offset_at == UNUSED_BUCKET {
            offset_at = Self::construct_rational(number);
        }

        for i in (last_defined + 1)..=nimber {
            let nus = Nus {
                number,
                up_multiple: 0,
                nimber: i,
            };
            let new_offset = self.write_nus_record_execpt_options(&nus, i as u32, i as u32);

            let prev_nimber_sector = &self.data[(offset_at >> SECTOR_BITS) as usize].clone();

            let sector = unsafe {
                self.data
                    .get_unchecked_mut((new_offset >> SECTOR_BITS) as usize)
            };
            let sector_offset = (new_offset as usize) & SECTOR_MASK;

            // Copy the options from the previous nimber to this one.
            copy_slice(
                prev_nimber_sector,
                ((offset_at as usize) & SECTOR_MASK) + 2,
                sector,
                sector_offset + 2,
                (i as usize) - 1,
            );

            sector[sector_offset + 2 + (i as usize) - 1] = offset_at;

            // Copy the left options as right options.
            copy_slice(
                &sector.clone(),
                sector_offset + 2,
                sector,
                sector_offset + 2 + (i as usize),
                i as usize,
            );

            let options_hash = Self::hash_options(
                i as u32,
                sector,
                (sector_offset as u32) + 2,
                i as u32,
                sector,
                (sector_offset as u32) + 2 + (i as u32),
            );

            self.write_to_index(options_hash, new_offset);
            offset_at = new_offset;
        }

        offset_at
    }

    pub fn new() -> Self {
        let mut data = Vec::with_capacity(DEFAULT_SECTOR_SLOTS);
        for _ in 0..DEFAULT_SECTOR_SLOTS {
            data.push(vec![0; SECTOR_SIZE]);
        }

        let mut res = Self {
            index_capacity: DEFAULT_INDEX_CAPACITY,
            index_mask: DEFAULT_INDEX_MASK,
            index: vec![UNUSED_BUCKET; DEFAULT_INDEX_CAPACITY],
            data,
            next_offset: 0,
            next_sector: 1,
            total_records: 0,
            op_table_op: vec![Operation::None; DEFAULT_OP_TABLE_SIZE],
            op_table_g: vec![0; DEFAULT_OP_TABLE_SIZE],
            op_table_h: vec![0; DEFAULT_OP_TABLE_SIZE],
            op_table_result: vec![0; DEFAULT_OP_TABLE_SIZE],
            zero_id: 3, // constructed by hand below
            star_id: 0, // Set below
        };

        // Construct 0 directly.  (It's a special case.)
        res.data[0][0] = UNUSED_BUCKET;
        res.data[0][1] = STD_NUS_RECORD;
        res.data[0][2] = 0;
        res.data[0][3] = UNUSED_BUCKET;
        res.data[0][4] = STD_OPTIONS_RECORD | IS_NUS_MASK;

        res.write_to_index(
            SmallNus {
                numerator: 0,
                den_exp: 0,
                up_multiple: 0,
                nimber: 0,
            }
            .hash(),
            0,
        );
        res.write_to_index(Self::hash_options(0, &Vec::new(), 0, 0, &Vec::new(), 0), 3);
        res.next_offset += 5;

        res.star_id = res.construct_nimber(DyadicRationalNumber::from(0), 1);

        res
    }
}

impl GameStorage {
    fn get_left_options_no(&self, game_id: u32) -> u32 {
        (self.data[(game_id >> SECTOR_BITS) as usize][((game_id + 1) as usize) & SECTOR_MASK]
            & NUM_LO_MASK)
            >> NUM_LO_SHIFT
    }

    pub fn get_left_options(&self, game_id: u32) -> impl Iterator<Item = u32> + '_ {
        let no_left_options = self.get_left_options_no(game_id);
        (0..no_left_options).map(move |idx| {
            self.data[(game_id >> SECTOR_BITS) as usize][(game_id + 2 + idx) as usize & SECTOR_MASK]
        })
    }

    fn get_right_options_no(&self, game_id: u32) -> u32 {
        self.data[(game_id >> SECTOR_BITS) as usize][((game_id + 1) as usize) & SECTOR_MASK]
            & NUM_RO_MASK
    }

    pub fn get_right_options(&self, game_id: u32) -> impl Iterator<Item = u32> + '_ {
        let no_left_options = self.get_left_options_no(game_id);
        let no_right_options = self.get_right_options_no(game_id);
        (0..no_right_options).map(move |idx| {
            self.data[(game_id >> SECTOR_BITS) as usize]
                [(game_id + 2 + no_left_options + idx) as usize & SECTOR_MASK]
                .clone()
        })
    }
}

#[test]
fn correct_star_options() {
    let gs = GameStorage::new();
    // * = {0|0}
    assert_eq!(
        gs.get_left_options(gs.star_id).collect::<Vec<_>>(),
        vec![gs.zero_id]
    );
    assert_eq!(
        gs.get_right_options(gs.star_id).collect::<Vec<_>>(),
        vec![gs.zero_id]
    );
}
