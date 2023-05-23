use std::{
    cmp::{self, Ordering},
    fmt::Display,
    io::Write,
    ops::Neg,
};

use itertools::Itertools;
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

#[repr(u8)]
#[derive(Debug, FromPrimitive, Clone, Copy, PartialEq, Eq)]
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

impl From<DyadicRationalNumber> for Nus {
    fn from(number: DyadicRationalNumber) -> Self {
        Nus {
            number,
            up_multiple: 0,
            nimber: 0,
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

impl From<i32> for SmallNus {
    fn from(numerator: i32) -> Self {
        SmallNus {
            numerator,
            den_exp: 0,
            up_multiple: 0,
            nimber: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Player {
    Left,
    Right,
}

/// Like arraycopy
fn copy_slice<T>(src: &[T], src_start: usize, dst: &mut [T], dst_start: usize, len: usize)
where
    T: Clone,
{
    let src_end = src_start + len;
    let es = &src[src_start..src_end];
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
    op_table_size: usize,
    op_table_mask: usize,
    op_table_op: Vec<Operation>,
    op_table_g: Vec<u32>,
    op_table_h: Vec<u32>,
    op_table_result: Vec<u32>,
    // known IDs
    pub zero_id: u32,
    pub star_id: u32,
    pub up_id: u32,
    pub up_star_id: u32,
    pub one_id: u32,
    pub negative_one_id: u32,
    pub two_id: u32,
    pub negative_two_id: u32,
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

    /// Dont' use it to construct 0 for the first time or it'll loop
    pub fn construct_integer(&mut self, val: i32) -> u32 {
        let num_lo = if val > 0 { 1 } else { 0 };
        let num_ro = if val < 0 { 1 } else { 0 };
        let sign = if val >= 0 { 1 } else { -1 };
        let mut offset_at;

        let mut last_defined = val;
        loop {
            offset_at = self.lookup_small_nus_record(&SmallNus::from(last_defined));
            if offset_at != UNUSED_BUCKET {
                break;
            }
            last_defined -= sign;
        }

        let mut i = last_defined + sign;
        loop {
            if i == val + sign {
                break;
            }

            let new_offset = self.write_nus_record_execpt_options(
                &Nus::from(DyadicRationalNumber::from(val)),
                num_lo,
                num_ro,
            );
            let sector = unsafe {
                self.data
                    .get_unchecked_mut((new_offset >> SECTOR_BITS) as usize)
            };
            let sector_offset = (new_offset as usize) & SECTOR_MASK;
            sector[sector_offset + 2] = offset_at;
            let options_hash = Self::hash_options(
                num_lo,
                sector,
                (sector_offset as u32) + 2,
                num_ro,
                sector,
                (sector_offset as u32) + 2,
            );
            self.write_to_index(options_hash, new_offset);
            offset_at = new_offset;
            i += sign;
        }

        offset_at
    }

    pub fn construct_rational(&mut self, number: DyadicRationalNumber) -> u32 {
        let nus = Nus::from(number);
        let offset = self.lookup_nus_record(&nus);
        if offset != UNUSED_BUCKET {
            return offset;
        }

        if let Some(int) = number.to_integer() {
            return self.construct_integer(int);
        }

	// These steps are missimplemented
        let left_option = self.construct_rational(number.step(-1));
        let right_option = self.construct_rational(number.step(1));
        let offset = self.write_nus_record_execpt_options(&Nus::from(number), 1, 1);
        let sector = unsafe {
            self.data
                .get_unchecked_mut((offset >> SECTOR_BITS) as usize)
        };
        let sector_offset = (offset as usize) & SECTOR_MASK;
        sector[sector_offset + 2] = left_option;
        sector[sector_offset + 3] = right_option;
        let options_hash = Self::hash_options(
            1,
            sector,
            (sector_offset as u32) + 2,
            1,
            sector,
            (sector_offset as u32) + 3,
        );
        self.write_to_index(options_hash, offset);
        return offset;
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

    pub fn construct_nus(&mut self, nus: &Nus) -> u32 {
        let parity = nus.up_multiple & 1;
        let sign = if nus.up_multiple >= 0 { 1 } else { -1 };
        let number_offset = self.construct_rational(nus.number);
        let mut last_defined = nus.up_multiple;
        let mut offset_at;

        loop {
            let tmp_nus = Nus {
                number: nus.number,
                up_multiple: last_defined,
                nimber: nus.nimber ^ parity ^ (last_defined & 1),
            };
            offset_at = self.lookup_nus_record(&tmp_nus);
            if offset_at != UNUSED_BUCKET || last_defined == 0 {
                break;
            }
            last_defined -= sign;
        }

        if offset_at == UNUSED_BUCKET {
            offset_at = self.construct_nimber(nus.number, nus.nimber ^ parity);
        }

        let mut i = last_defined + sign;
        loop {
            if i == nus.up_multiple + sign {
                break;
            }

            let num_lo;
            let num_ro;
            let mut star_offset = 0;
            let current_nimber = nus.nimber ^ parity ^ (i & 1);

            if i == 1 && current_nimber == 1 {
                // special case for n^*
                star_offset = self.construct_nus(&Nus {
                    number: nus.number,
                    up_multiple: 0,
                    nimber: 1,
                });
                num_lo = 2;
                num_ro = 1;
            } else if i == -1 && current_nimber == 1 {
                // special case for nv*
                star_offset = self.construct_nus(&Nus {
                    number: nus.number,
                    up_multiple: 0,
                    nimber: 1,
                });
                num_lo = 1;
                num_ro = 2;
            } else {
                num_lo = 1;
                num_ro = 1;
            }

            let new_offset = self.write_nus_record_execpt_options(
                &Nus {
                    number: nus.number,
                    up_multiple: i,
                    nimber: current_nimber,
                },
                num_lo,
                num_ro,
            );
            let sector = unsafe {
                self.data
                    .get_unchecked_mut((new_offset >> SECTOR_BITS) as usize)
            };
            let sector_offset = (new_offset as usize) & SECTOR_MASK;

            if i == 1 && current_nimber == 1 {
                sector[sector_offset + 2] = number_offset;
                sector[sector_offset + 3] = star_offset;
                sector[sector_offset + 4] = number_offset;
            } else if i == -1 && current_nimber == 1 {
                sector[sector_offset + 2] = number_offset;
                sector[sector_offset + 3] = number_offset;
                sector[sector_offset + 4] = star_offset;
            } else if i > 0 {
                sector[sector_offset + 2] = number_offset;
                sector[sector_offset + 3] = offset_at;
            } else {
                sector[sector_offset + 2] = offset_at;
                sector[sector_offset + 3] = number_offset;
            }

            let options_hash = Self::hash_options(
                num_lo,
                sector,
                (sector_offset as u32) + 2,
                num_ro,
                sector,
                (sector_offset as u32) + 2 + num_lo,
            );
            self.write_to_index(options_hash, new_offset);

            offset_at = new_offset;
            i += sign;
        }

        offset_at
    }

    pub fn construct_nimber(&mut self, number: DyadicRationalNumber, nimber: i32) -> u32 {
        let mut last_defined: i32 = nimber;
        let mut offset_at: u32;
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
            offset_at = self.construct_rational(number);
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

    /// Function is mutable to store the cache result
    fn birthday(&mut self, id: u32) -> i32 {
        if self.is_number_up_star(id) {
            let den_exp = self.get_den_exp_part(id);
            let up_mag = self.get_up_multiple_part(id).abs();
            let nimber = self.get_nimber_part(id);

            let number_birthday: i32;
            if self.is_extended_record(id) {
                todo!()
            } else {
                let num_mag = self.get_small_numerator_part(id).abs();
                number_birthday = if den_exp == 0 {
                    num_mag
                } else {
                    1 + (num_mag >> den_exp) as i32 + den_exp as i32
                }
            }

            let up_star_birthday: i32 = if up_mag > 0 && nimber == 0 {
                up_mag + 1
            } else if (up_mag & 1) == 1 && nimber != 1 {
                up_mag + (nimber ^ 1)
            } else {
                up_mag + nimber
            };
            return number_birthday + up_star_birthday;
        }
        let birthday = self.lookup_op_result(Operation::Birthday, id, -1i32 as u32);
        if let Some(birthday) = birthday {
            return birthday as i32;
        }
        let mut birthday: i32 = 0;

        // We need to `collect` or borrow checker will complain
        for left_opt in self.get_left_options(id).collect::<Vec<_>>() {
            birthday = cmp::max(birthday, self.birthday(left_opt) + 1);
        }
        for right_opt in self.get_right_options(id).collect::<Vec<_>>() {
            birthday = cmp::max(birthday, self.birthday(right_opt) + 1);
        }

        self.store_op_result(Operation::Birthday, id, -1i32 as u32, birthday as u32);
        birthday
    }

    fn get_small_numerator_part(&self, id: u32) -> i32 {
        self.data[(id >> SECTOR_BITS) as usize][(id as usize - 1) & SECTOR_MASK] as i32
    }

    fn get_den_exp_part(&self, id: u32) -> u32 {
        if self.is_extended_record(id) {
            let offset = self.get_extended_nus_record_offset(id);
            self.data[(offset >> SECTOR_BITS) as usize][(offset as usize + 1) & SECTOR_MASK]
                & EXT_DENOMINATOR_MASK
        } else {
            (self.data[(id >> SECTOR_BITS) as usize][(id as usize - 2) & SECTOR_MASK]
                & DENOMINATOR_MASK)
                >> DENOMINATOR_SHIFT
        }
    }

    pub fn get_number_part(&self, id: u32) -> DyadicRationalNumber {
        if self.is_extended_record(id) {
            todo!()
        } else {
            DyadicRationalNumber::rational(
                self.get_small_numerator_part(id),
                1 << self.get_den_exp_part(id),
            )
            .unwrap()
        }
    }

    pub fn get_up_multiple_part(&self, id: u32) -> i32 {
        if self.is_extended_record(id) {
            let offset = self.get_extended_nus_record_offset(id);
            self.data[(offset >> SECTOR_BITS) as usize][(offset as usize + 3) & SECTOR_MASK] as i32
        } else {
            ((self.data[(id >> SECTOR_BITS) as usize][(id as usize - 2) & SECTOR_MASK]
                << UP_MULTIPLE_LEFTSHIFT)
                >> UP_MULTIPLE_RIGHTSHIFT) as i32
        }
    }

    fn is_extended_record(&self, id: u32) -> bool {
        (self.data[(id >> SECTOR_BITS) as usize][(id as usize + 1) & SECTOR_MASK] & EXT_RECORD_MASK)
            != 0
    }

    fn get_extended_nus_record_offset(&self, id: u32) -> u32 {
        id + 2 + self.get_left_options_no(id) + self.get_right_options_no(id)
    }

    pub fn get_nimber_part(&self, id: u32) -> i32 {
        if self.is_extended_record(id) {
            let offset = self.get_extended_nus_record_offset(id);
            self.data[(offset >> SECTOR_BITS) as usize][(offset as usize + 4) & SECTOR_MASK] as i32
        } else {
            (self.data[(id >> SECTOR_BITS) as usize][(id as usize - 2) & SECTOR_MASK] & NIMBER_MASK)
                as i32
        }
    }

    fn is_nimber(&self, id: u32) -> bool {
        self.is_number_up_star(id)
            && self.get_number_part(id) == DyadicRationalNumber::from(0)
            && self.get_up_multiple_part(id) == 0
    }

    pub fn is_number_up_star(&self, id: u32) -> bool {
        (self.data[(id >> SECTOR_BITS) as usize][(id as usize + 1) & SECTOR_MASK] & IS_NUS_MASK)
            != 0
    }

    pub fn is_number(&self, id: u32) -> bool {
        self.is_number_up_star(id)
            && self.get_nimber_part(id) == 0
            && self.get_up_multiple_part(id) == 0
    }

    // pub fn is_number_tiny(&self, id: u32) -> bool {
    //     let lo: Vec<u32> = self.get_left_options(id).collect();
    //     let ro: Vec<u32> = self.get_right_options(id).collect();

    //     lo.len() == 1
    //         && ro.len() == 1
    //         && (self.is_number(lo[0]) && {
    //             let rlo: Vec<u32> = self.get_left_options(ro[0]).collect();
    //             let rro: Vec<u32> = self.get_right_options(ro[0]).collect();
    //             rlo.len() == 1
    //                 && rro.len() == 1
    //                 && lo[0] == rlo[0]
    //                 && self.mean(rro[0]) < self.mean(lo[0])
    //         })
    // }

    // pub fn mean(&self, id: u32) -> DyadicRationalNumber {
    // 	if self.is_number_up_star(id) {
    // 	    self.get_number_part(id)
    // 	} else {

    // 	}
    // }

    fn lookup_op_result(&self, operation: Operation, gid: u32, hid: u32) -> Option<u32> {
        let operation_s = operation as u32;
        let hc: usize = ((operation_s ^ gid ^ hid) as usize) & self.op_table_mask;
        if self.op_table_op[hc] == operation
            && (self.op_table_g[hc] == gid && self.op_table_h[hc] == hid
                || operation == Operation::Sum
                    && self.op_table_g[hc] == hid
                    && self.op_table_h[hc] == gid)
        {
            Some(self.op_table_result[hc])
        } else {
            None
        }
    }

    fn lookup_options_record(&self, left_options: &[u32], right_options: &[u32]) -> u32 {
        let num_lo = left_options.len();
        let num_ro = right_options.len();
        let hashed_options = Self::hash_options(
            num_lo as u32,
            left_options,
            0,
            num_ro as u32,
            right_options,
            0,
        );
        let mut offset_at = self.index[hashed_options as usize & self.index_mask];

        while offset_at != UNUSED_BUCKET {
            let sector = &self.data[(offset_at >> SECTOR_BITS) as usize];
            let sector_offset = offset_at as usize & SECTOR_MASK;
            let descriptor = sector[sector_offset + 1];
            if (descriptor & NUS_RECORD_MASK) == 0 {
                let mut matches = num_lo == ((descriptor & NUM_LO_MASK) >> NUM_LO_SHIFT) as usize
                    && num_ro == (descriptor & NUM_RO_MASK) as usize;

                if matches {
                    for i in 0..num_lo {
                        if left_options[i] != sector[sector_offset + 2 + i as usize] {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        for i in 0..num_ro {
                            if right_options[i] != sector[sector_offset + 2 + num_lo + i as usize] {
                                matches = false;
                                break;
                            }
                        }
                    }
                }

                if matches {
                    break;
                }
            }
            offset_at = sector[sector_offset];
        }

        offset_at
    }

    fn compare_number_parts(&self, gid: u32, hid: u32) -> i32 {
        if (self.data[(gid >> SECTOR_BITS) as usize][(gid as usize + 1) & SECTOR_MASK]
            & EXT_RECORD_MASK)
            != 0
            || (self.data[(hid >> SECTOR_BITS) as usize][(hid as usize + 1) & SECTOR_MASK]
                & EXT_RECORD_MASK)
                != 0
        {
            // At least one of the numbers is large.
            // self.get_number_part(gid).compare_to(self.get_number_part(hid))
            todo!()
        } else {
            let g_num = self.get_small_numerator_part(gid);
            let g_den_exp = self.get_den_exp_part(gid);
            let h_num = self.get_small_numerator_part(hid);
            let h_den_exp = self.get_den_exp_part(hid);
            let cmp: i64;

            if g_den_exp <= h_den_exp {
                cmp = ((g_num as i64) << (h_den_exp - g_den_exp)) - (h_num as i64);
            } else {
                cmp = (g_num as i64) - ((h_num as i64) << (g_den_exp - h_den_exp));
            }

            cmp.signum() as i32
        }
    }

    // This function ASSUMES that the supplied arrays contain no dominated or reversible options.
    // Passing unsimplified arrays to this method will "seriously screw up everything"
    fn construct_as_nus_entry(&mut self, left_options: &[u32], right_options: &[u32]) -> u32 {
        let number: DyadicRationalNumber;
        let up_multiple: i32;
        let nimber: i32;

        let num_lo = left_options.len();
        let num_ro = right_options.len();

        if num_lo == 0 {
            if num_ro == 0 {
                number = DyadicRationalNumber::from(0);
            } else {
                // We assume that entry is normalized, no left options, thus there must be only one
                // right entry that's a number
                assert!(num_ro == 1, "Entry not normalized");
                number = self.get_number_part(right_options[0]) - DyadicRationalNumber::from(1);
            }
            up_multiple = 0;
            nimber = 0;
        } else if num_ro == 0 {
            assert!(num_lo == 1, "Entry not normalized");
            number = self.get_number_part(left_options[0]) + DyadicRationalNumber::from(1);
            up_multiple = 0;
            nimber = 0;
        } else if num_lo == 1
            && num_ro == 1
            && self.is_number(left_options[0])
            && self.is_number(right_options[0])
            && self.compare_number_parts(left_options[0], right_options[0]) < 0 
        {
            // We're a number but not an integer.  Conveniently, since the
            // option lists are canonicalized, the value of this game is the
            // mean of its left & right options.
            number = self
                .get_number_part(left_options[0])
                .mean(&self.get_number_part(right_options[0]));
            up_multiple = 0;
            nimber = 0;
        } else if num_lo == 2
            && num_ro == 1
            && self.is_number(left_options[0])
            && left_options[0] == right_options[0]
            && self.is_number_up_star(left_options[1])
            && self.compare_number_parts(left_options[0], left_options[1]) == 0
            && self.get_up_multiple_part(left_options[1]) == 0
            && self.get_nimber_part(left_options[1]) == 1
        {
            // For some number n, the form of this game is {n,n*|n} = n^*.
            number = self.get_number_part(left_options[0]);
            up_multiple = 1;
            nimber = 1;
        } else if num_lo == 1
            && num_ro == 2
            && self.is_number(left_options[0])
            && left_options[0] == right_options[0]
            && self.is_number_up_star(right_options[1])
            && self.compare_number_parts(right_options[0], right_options[1]) == 0
            && self.get_up_multiple_part(right_options[1]) == 0
            && self.get_nimber_part(right_options[1]) == 1
        {
            // Flip of the previous one
            number = self.get_number_part(right_options[0]);
            up_multiple = -1;
            nimber = 1; 
        } else if num_lo == 1
            && num_ro == 1
            && self.is_number(left_options[0])
            && self.is_number_up_star(right_options[0])
            && !self.is_number(right_options[0])
            && self.compare_number_parts(left_options[0], right_options[0]) == 0
            && self.get_up_multiple_part(right_options[0]) >= 0
        {
            // This is of the form n + {0|G} where G is a number-up-star of up multiple >= 0.
            number = self.get_number_part(left_options[0]);
            up_multiple = self.get_up_multiple_part(right_options[0]) + 1;
            nimber = self.get_nimber_part(right_options[0]) ^ 1;
        } else if num_lo == 1
            && num_ro == 1
            && self.is_number(right_options[0])
            && self.is_number_up_star(left_options[0])
            && !self.is_number(left_options[0])
            && self.compare_number_parts(left_options[0], right_options[0]) == 0
            && self.get_up_multiple_part(left_options[0]) <= 0
        {
            // This is of the form n + {G|0} where G is a number-up-star of up multiple <= 0.
            // Flip of the previous one
            number = self.get_number_part(left_options[0]);
            up_multiple = self.get_up_multiple_part(left_options[0]) - 1;
            nimber = self.get_nimber_part(left_options[0]) ^ 1;
        } else if num_lo >= 1 // NOTE: HERE
            && num_ro >= 1
            && num_lo == num_ro
            && self.is_number(left_options[0])
            && left_options[0] == right_options[0]
        {
            // Last we need to check for games of the form n + *k.
            for i in 0..num_lo {
                let l = left_options[i];
                let r = right_options[i];
                if l != r
                    || self.is_number_up_star(l)
                    || self.compare_number_parts(l, r) != 0
                    || self.get_up_multiple_part(l) != 0
                    || self.get_nimber_part(l) != (i as i32)
                {
                    return UNUSED_BUCKET;
                }
            }
            // It's a nimber.
            number = self.get_number_part(left_options[0]);
            up_multiple = 0;
            nimber = num_lo as i32;
        } else {
            return UNUSED_BUCKET;
        }

        // It's a nus
        let nus = Nus {
            number,
            up_multiple,
            nimber,
        };

        let offset = self.write_nus_record_execpt_options(&nus, num_lo as u32, num_ro as u32);
        let sector = unsafe {
            self.data
                .get_unchecked_mut((offset >> SECTOR_BITS) as usize)
        };
        let sector_offset = (offset & SECTOR_MASK as u32) as usize;
        copy_slice(left_options, 0, sector, sector_offset + 2, num_lo);
        copy_slice(right_options, 0, sector, sector_offset + 2 + num_lo, num_ro);

        let options_hash = Self::hash_options(
            num_lo as u32,
            sector,
            (sector_offset + 2) as u32,
            num_ro as u32,
            sector,
            (sector_offset + 2 + num_lo) as u32,
        );
        self.write_to_index(options_hash, offset);

        offset
    }

    /// This function ASSUMES that the supplied arrays contain no dominated or reversible options.
    /// Passing unsimplified arrays to this method will "seriously screw up everything"
    fn construct_from_canonical_options(
        &mut self,
        mut left_options: Vec<u32>,
        mut right_options: Vec<u32>,
    ) -> u32 {
        left_options.sort();
        right_options.sort();

        let offset = self.lookup_options_record(&left_options, &right_options);
        if offset != UNUSED_BUCKET {
            return offset;
        }

        let offset = self.construct_as_nus_entry(&left_options, &right_options);
        if offset != UNUSED_BUCKET {
            return offset;
        }

        let entry_size = (2 + left_options.len() + right_options.len()) as u32;
        self.ensure_sector_space(entry_size);

        let offset = self.next_offset;
        self.next_offset += entry_size;

        let sector = unsafe {
            self.data
                .get_unchecked_mut((offset >> SECTOR_BITS) as usize)
        };
        let sector_offset = (offset as usize) & SECTOR_MASK;

        sector[sector_offset] = UNUSED_BUCKET;
        sector[sector_offset + 1] = STD_OPTIONS_RECORD
            | (left_options.len() << NUM_LO_SHIFT) as u32
            | right_options.len() as u32;

        copy_slice(
            &left_options,
            0,
            sector,
            sector_offset + 2,
            left_options.len(),
        );
        copy_slice(
            &right_options,
            0,
            sector,
            sector_offset + 2 + left_options.len(),
            right_options.len(),
        );

        let option_hash = Self::hash_options(
            left_options.len() as u32,
            &left_options,
            0,
            right_options.len() as u32,
            &right_options,
            0,
        );
        self.write_to_index(option_hash, offset);

        offset
    }

    fn store_op_result(&mut self, operation: Operation, gid: u32, hid: u32, result: u32) {
        let operation_s = operation as u32;
        let hc: usize = ((operation_s ^ gid ^ hid) as usize) & self.op_table_mask;
        self.op_table_op[hc] = operation;
        self.op_table_g[hc] = gid;
        self.op_table_h[hc] = hid;
        self.op_table_result[hc] = result;
    }

    fn eliminate_duplicate_options(gs: &[u32]) -> Vec<u32> {
        gs.iter().sorted().dedup().cloned().collect()
    }

    fn mex(&self, opts: &[u32]) -> i32 {
        let mut i = 0;
        let mut mex = 0;

        for opt in opts {
            i += 1;
            if !self.is_nimber(*opt) {
                // FIXME: Use None
                return -1;
            } else if self.get_nimber_part(*opt) == mex {
                mex += 1;
            } else {
                // It's a nimber, but exceeds mex.  We've found the true
                // mex - *provided* everything that remains is a nimber.
                break;
            }
        }

        for opt in opts[i..].iter() {
            if !self.is_nimber(*opt) {
                return -1;
            }
        }

        mex
    }

    pub fn construct_from_options(&mut self, ls: &[u32], rs: &[u32]) -> u32 {
        let ls = Self::eliminate_duplicate_options(ls);
        let rs = Self::eliminate_duplicate_options(rs);

        let left_mex = self.mex(&ls);
        if left_mex >= 0 {
            let right_mex = self.mex(&rs);
            if left_mex == right_mex {
                let nus = Nus {
                    number: DyadicRationalNumber::from(0),
                    up_multiple: 0,
                    nimber: left_mex,
                };
                return self.construct_nus(&nus);
            }
        }

        let ls = self.bypass_reversible_options_l(ls, &rs);
        let rs = self.bypass_reversible_options_r(&ls, rs);

        let ls = self.eliminate_dominated_options(&ls, true);
        let rs = self.eliminate_dominated_options(&rs, false);

        self.construct_from_canonical_options(ls, rs)
    }

    fn eliminate_dominated_options(
        &mut self,
        options: &[u32],
        eliminate_smaller_options: bool,
    ) -> Vec<u32> {
        let mut options = options.to_vec();
        for i in 0..options.len() {
            if options[i] == -1i32 as u32 {
                continue;
            }
            for j in 0..i {
                if options[j] == -1i32 as u32 {
                    continue;
                }
                if eliminate_smaller_options && self.leq(options[i], options[j])
                    || !eliminate_smaller_options && self.leq(options[j], options[i])
                {
                    options[i] = -1i32 as u32;
                    break;
                }
                if eliminate_smaller_options && self.leq(options[j], options[i])
                    || !eliminate_smaller_options && self.leq(options[i], options[j])
                {
                    options[j] = -1i32 as u32;
                }
            }
        }
        Self::pack(&options)
    }

    fn pack(options: &[u32]) -> Vec<u32> {
        options
            .iter()
            .cloned()
            .filter(|o| *o != -1i32 as u32)
            .collect()
    }

    fn bypass_reversible_options_l(
        &mut self,
        mut left_options: Vec<u32>,
        right_options: &[u32],
    ) -> Vec<u32> {
        let mut i: i64 = 0;
        loop {
            if (i as usize) >= left_options.len() {
                break;
            }
            if left_options[i as usize] == -1i32 as u32 {
                continue;
            }
            let g_lrs = self
                .get_right_options(left_options[i as usize])
                .collect::<Vec<_>>();
            for j in 0..self.get_right_options_no(left_options[i as usize]) {
                let g_lr = g_lrs[j as usize];
                if self.leq_arrays(g_lr, &left_options, right_options) {
                    let mut new_left_options =
                        vec![
                            -1i32 as u32;
                            left_options.len() + self.get_left_options_no(g_lr) as usize - 1
                        ];
                    for k in 0..(i as usize) {
                        new_left_options[k] = left_options[k];
                    }
                    for k in (i as usize + 1)..left_options.len() {
                        new_left_options[k - 1] = left_options[k];
                    }
                    for (k, g_lrl) in self.get_left_options(g_lr).enumerate() {
                        if left_options.contains(&g_lrl) {
                            new_left_options[left_options.len() + k - 1] = -1i32 as u32;
                        } else {
                            new_left_options[left_options.len() + k - 1] = g_lrl;
                        }
                    }
                    left_options = new_left_options;
                    i -= 1;
                    break;
                }
            }
            i += 1;
        }
        Self::pack(&left_options)
    }

    fn bypass_reversible_options_r(
        &mut self,
        left_options: &[u32],
        mut right_options: Vec<u32>,
    ) -> Vec<u32> {
        let mut i: i64 = 0;
        loop {
            if (i as usize) >= right_options.len() {
                break;
            }
            if right_options[i as usize] == -1i32 as u32 {
                continue;
            }
            let g_rls = self
                .get_left_options(right_options[i as usize])
                .collect::<Vec<_>>();
            for j in 0..self.get_left_options_no(right_options[i as usize]) {
                let g_rl = g_rls[j as usize];
                if self.geq_arrays(g_rl, left_options, &right_options) {
                    let mut new_right_options =
                        vec![
                            -1i32 as u32;
                            right_options.len() + self.get_right_options_no(g_rl) as usize - 1
                        ];
                    for k in 0..(i as usize) {
                        new_right_options[k] = right_options[k];
                    }
                    for k in (i as usize + 1)..right_options.len() {
                        new_right_options[k - 1] = right_options[k];
                    }
                    for (k, g_rlr) in self.get_right_options(g_rl).enumerate() {
                        if right_options.contains(&g_rlr) {
                            new_right_options[right_options.len() + k - 1] = -1i32 as u32;
                        } else {
                            new_right_options[right_options.len() + k - 1] = g_rlr;
                        }
                    }
                    right_options = new_right_options;
                    i -= 1;
                    break;
                }
            }
            i += 1;
        }
        Self::pack(&right_options)
    }

    /// Return false if `H <= GL` for some left option `GL` of `G` or `HR <= G` for some right
    /// option `HR` of `H`. Otherwise return true.
    fn leq_arrays(&mut self, id: u32, ls: &[u32], rs: &[u32]) -> bool {
        for r_opt in rs {
            // NOTE: Aaron uses different comparasion function here
            if self.leq(*r_opt, id) {
                return false;
            }
        }

        let sector = &self.data[(id >> SECTOR_BITS) as usize].clone();
        let sector_offset = (id as usize) & SECTOR_MASK;
        let num_lo = (sector[sector_offset + 1] & NUM_LO_MASK) >> NUM_LO_SHIFT;

        for i in 0..num_lo {
            if self.geq_arrays(sector[sector_offset + 2 + i as usize], ls, rs) {
                return false;
            }
        }

        return true;
    }

    /// Return false if `GR <= H` or `G <= HL`. Otherwise return true.
    fn geq_arrays(&mut self, id: u32, ls: &[u32], rs: &[u32]) -> bool {
        for l_opt in ls {
            if self.leq(id, *l_opt) {
                return false;
            }
        }

        let sector = &self.data[(id >> SECTOR_BITS) as usize].clone();
        let sector_offset = (id as usize) & SECTOR_MASK;
        let num_lo = (sector[sector_offset + 1] & NUM_LO_MASK) >> NUM_LO_SHIFT;
        let num_ro = sector[sector_offset + 1] & NUM_RO_MASK;

        for i in 0..num_ro {
            if self.leq_arrays(
                sector[sector_offset + 2 + num_lo as usize + i as usize],
                ls,
                rs,
            ) {
                return false;
            }
        }

        return true;
    }

    pub fn add(&mut self, gid: u32, hid: u32) -> u32 {
        if self.is_number_up_star(gid) && self.is_number_up_star(hid) {
            if self.is_extended_record(gid) || self.is_extended_record(hid) {
                todo!()
            } else {
                let g_num = self.get_small_numerator_part(gid);
                let g_den = self.get_den_exp_part(gid);

                let h_num = self.get_small_numerator_part(hid);
                let h_den = self.get_den_exp_part(hid);

                let mut sum_den;
                let mut sum_num;

                if g_den >= h_den {
                    sum_den = g_den;
                    sum_num = g_num + (h_num << (g_den - h_den));
                } else {
                    sum_den = h_den;
                    sum_num = h_num + (g_num << (h_den - g_den));
                }

                // If numerator is even we can half it and reduce exponent of denominator
                while (sum_num & 1) == 0 && sum_den > 0 {
                    sum_num >>= 1;
                    sum_den -= 1;
                }

                let nus = Nus {
                    number: DyadicRationalNumber::rational(sum_num, 1 << sum_den).unwrap(),
                    up_multiple: self.get_up_multiple_part(gid) + self.get_up_multiple_part(hid),
                    nimber: self.get_nimber_part(gid) ^ self.get_nimber_part(hid),
                };
                return self.construct_nus(&nus);
            }
        }
        if let Some(result) = self.lookup_op_result(Operation::Sum, gid, hid) {
            return result;
        }

        let h_start_left_opts = if self.is_number(gid) {
            0
        } else {
            self.get_left_options_no(gid)
        };

        let h_start_right_opts = if self.is_number(gid) {
            0
        } else {
            self.get_right_options_no(gid)
        };

        let g_start_left_opts = if self.is_number(hid) {
            0
        } else {
            self.get_left_options_no(hid)
        };

        let g_start_right_opts = if self.is_number(hid) {
            0
        } else {
            self.get_right_options_no(hid)
        };

        let mut new_left_opts = vec![0; (h_start_left_opts + g_start_left_opts) as usize];
        let mut new_right_opts = vec![0; (h_start_right_opts + g_start_right_opts) as usize];

        if !self.is_number(gid) {
            for (idx, l) in self.get_left_options(gid).enumerate().collect::<Vec<_>>() {
                new_left_opts[idx] = self.add(l, hid);
            }
            for (idx, r) in self.get_right_options(gid).enumerate().collect::<Vec<_>>() {
                new_right_opts[idx] = self.add(r, hid);
            }
        }

        if !self.is_number(hid) {
            for (idx, l) in self.get_left_options(hid).enumerate().collect::<Vec<_>>() {
                new_left_opts[idx + h_start_left_opts as usize] = self.add(gid, l);
            }
            for (idx, r) in self.get_right_options(hid).enumerate().collect::<Vec<_>>() {
                new_right_opts[idx + h_start_right_opts as usize] = self.add(gid, r);
            }
        }

        let result = self.construct_from_options(&new_left_opts, &new_right_opts);
        self.store_op_result(Operation::Sum, gid, hid, result);
        result
    }

    pub fn get_negative(&mut self, id: u32) -> u32 {
        if self.is_number_up_star(id) {
            let nus = Nus {
                number: -self.get_number_part(id),
                up_multiple: -self.get_up_multiple_part(id),
                nimber: self.get_nimber_part(id),
            };
            return self.construct_nus(&nus);
        }

        if let Some(result) = self.lookup_op_result(Operation::Negative, id, -1i32 as u32) {
            return result;
        }

        // We have to do `collect` and `iter` to convince borrow checker that it's fine
        // NOTE: left and right are swapped on purpose
        let new_left_options: Vec<u32> = self
            .get_right_options(id)
            .collect::<Vec<_>>()
            .iter()
            .map(|opt| self.get_negative(*opt))
            .collect();
        let new_right_options: Vec<u32> = self
            .get_left_options(id)
            .collect::<Vec<_>>()
            .iter()
            .map(|opt| self.get_negative(*opt))
            .collect();

        let result = self.construct_from_canonical_options(new_left_options, new_right_options);
        self.store_op_result(Operation::Negative, id, -1i32 as u32, result);
        result
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
            op_table_size: DEFAULT_OP_TABLE_SIZE,
            op_table_mask: DEFAULT_OP_TABLE_MASK,
            op_table_op: vec![Operation::None; DEFAULT_OP_TABLE_SIZE],
            op_table_g: vec![0; DEFAULT_OP_TABLE_SIZE],
            op_table_h: vec![0; DEFAULT_OP_TABLE_SIZE],
            op_table_result: vec![0; DEFAULT_OP_TABLE_SIZE],
            zero_id: 3, // constructed by hand below
            star_id: 0,
            up_id: 0,
            up_star_id: 0,
            one_id: 0,
            negative_one_id: 0,
            two_id: 0,
            negative_two_id: 0, // Set below
        };

        // Don't use `construct_integer`
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

        let up = Nus {
            number: DyadicRationalNumber::from(0),
            up_multiple: 1,
            nimber: 0,
        };
        res.up_id = res.construct_nus(&up);

        let up_star = Nus {
            number: DyadicRationalNumber::from(0),
            up_multiple: 1,
            nimber: 1,
        };
        res.up_star_id = res.construct_nus(&up_star);

        res.one_id = res.construct_integer(1);
        res.negative_one_id = res.construct_integer(-1);
        res.two_id = res.construct_integer(2);
        res.negative_two_id = res.construct_integer(-2);

        res
    }
}

#[test]
fn constructs_integers() {
    let mut gs = GameStorage::new();
    assert_eq!(gs.construct_integer(4), 33);
    assert_eq!(gs.construct_integer(0x1000), 24585);
    assert_eq!(gs.construct_integer(-3), 24603);
}

#[test]
fn constructs_rationals() {
    let mut gs = GameStorage::new();
    assert_eq!(
        gs.construct_rational(DyadicRationalNumber::rational(1, 2).unwrap()),
        21
    );
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

    pub fn sorted_left_options(&mut self, game_id: u32) -> Vec<u32> {
        let mut options = self.get_left_options(game_id).collect::<Vec<_>>();
        options.sort_by(|gid, hid| self.compare(*gid, *hid));
        options
    }

    pub fn sorted_right_options(&mut self, game_id: u32) -> Vec<u32> {
        let mut options = self.get_right_options(game_id).collect::<Vec<_>>();
        options.sort_by(|gid, hid| self.compare(*gid, *hid));
        options
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

impl GameStorage {
    /// Inputs MUST be equal length
    fn compare_opts(&mut self, gs: &[u32], hs: &[u32]) -> Ordering {
        assert!(gs.len() == gs.len(), "Inputs are not equal length");
        for (g, h) in gs.iter().zip(hs) {
            let cmp = self.compare(*g, *h);
            match cmp {
                Ordering::Equal => continue,
                _ => return cmp,
            }
        }
        Ordering::Equal
    }

    /// Function is mutable because cache stuff
    fn compare(&mut self, gid: u32, hid: u32) -> Ordering {
        if gid == hid {
            return Ordering::Equal;
        }

        let mut cmp: i32 = self.birthday(gid) - self.birthday(hid);
        if cmp != 0 {
            return Ord::cmp(&cmp, &0);
        }

        cmp = self.get_left_options_no(gid) as i32 - self.get_left_options_no(hid) as i32;
        if cmp != 0 {
            return Ord::cmp(&cmp, &0);
        }

        cmp = self.get_right_options_no(gid) as i32 - self.get_right_options_no(hid) as i32;
        if cmp != 0 {
            return Ord::cmp(&cmp, &0);
        }

        let ls = self.sorted_left_options(gid);
        let rs = self.sorted_left_options(hid);
        let cmp = self.compare_opts(&ls, &rs);
        if cmp != Ordering::Equal {
            return cmp;
        }

        let ls = self.sorted_right_options(gid);
        let rs = self.sorted_right_options(hid);
        let cmp = self.compare_opts(&ls, &rs);
        if cmp != Ordering::Equal {
            return cmp;
        }

        dbg!(gid, self.game_to_str(gid), hid, self.game_to_str(hid));
        panic!("compare: Unreachable")
    }

    pub fn leq(&mut self, gid: u32, hid: u32) -> bool {
        match self.compare(gid, hid) {
            Ordering::Less => true,
            Ordering::Equal => true,
            Ordering::Greater => false,
        }
    }
}

#[test]
fn compare_works() {
    let mut gs = GameStorage::new();
    assert_eq!(gs.compare(gs.zero_id, gs.zero_id), Ordering::Equal);
    assert_eq!(gs.compare(gs.star_id, gs.star_id), Ordering::Equal);
    assert_eq!(gs.compare(gs.zero_id, gs.star_id), Ordering::Less);
    assert_eq!(gs.compare(gs.star_id, gs.zero_id), Ordering::Greater);
}

#[test]
fn addition_works() {
    let mut gs = GameStorage::new();

    let zero = gs.construct_integer(0);
    assert_eq!(gs.add(zero, zero), zero);

    let one = gs.construct_integer(1);
    let two = gs.construct_integer(2);
    assert_eq!(gs.add(one, one), two);

    let minus_one = gs.construct_integer(-1);
    assert_eq!(gs.add(one, minus_one), zero);

    let half = gs.construct_rational(DyadicRationalNumber::rational(1, 2).unwrap());
    assert_eq!(gs.add(half, half), one);
}

impl GameStorage {
    pub fn display_game<W>(&mut self, game_id: u32, f: &mut W) -> Result<(), std::fmt::Error>
    where
        W: std::fmt::Write,
    {
        if self.zero_id == game_id {
            write!(f, "0")?;
            return Ok(());
        }

        if self.is_number(game_id) {
            let num = self.get_number_part(game_id);
            write!(f, "{}", num)?;
            return Ok(());
        }

        let lo = self.sorted_left_options(game_id);
        let ro = self.sorted_right_options(game_id);

        write!(f, "{{")?;
        for (idx, l) in lo.iter().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            self.display_game(*l, f)?;
        }

        write!(f, "|")?;

        for (idx, r) in ro.iter().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            self.display_game(*r, f)?;
        }
        write!(f, "}}")?;

        Ok(())
    }

    fn game_to_str(&mut self, game_id: u32) -> String {
        let mut buf = String::new();
        self.display_game(game_id, &mut buf).unwrap();
        buf
    }
}

#[test]
fn pretty_printing_works() {
    let mut gs = GameStorage::new();

    let mut buf = String::new();
    let zero = gs.construct_integer(0);
    gs.display_game(zero, &mut buf).unwrap();
    assert_eq!(buf, "0".to_string());

    let mut buf = String::new();
    let fourty_two = gs.construct_integer(42);
    gs.display_game(fourty_two, &mut buf).unwrap();
    assert_eq!(buf, "42".to_string());

    let mut buf = String::new();
    let fourty_two = gs.construct_rational(DyadicRationalNumber::rational(1, 4).unwrap());
    gs.display_game(fourty_two, &mut buf).unwrap();
    assert_eq!(buf, "1/4".to_string());

    let mut buf = String::new();
    let star = gs.construct_nimber(DyadicRationalNumber::from(0), 1);
    gs.display_game(star, &mut buf).unwrap();
    assert_eq!(buf, "{0|0}".to_string());

    let mut buf = String::new();
    let star = gs.construct_nimber(DyadicRationalNumber::from(0), 2);
    gs.display_game(star, &mut buf).unwrap();
    assert_eq!(buf, "{0, {0|0}|0, {0|0}}".to_string());
}
