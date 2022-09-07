use std::marker::PhantomData;

use crate::*;

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct RawValue {
    data: Vec<u8>,
    bit_width: u8
}

impl RawValue {
    pub fn new(value: Vec<u8>, bit_width: u8) -> Self {
        Self {
            data: value,
            bit_width
        }
    }

    pub fn get_val(&self, index: u8)-> Option<u8> {
        let block_step = u8::BITS / u8::BITS + self.bit_width as u32 / u8::BITS;
        let w = index as u32 / u8::BITS * block_step;
        let b = index as u32 % u8::BITS;
        let flag = (1 as u8) << b;
        if self.data.get((w + block_step - 1) as usize).is_none() {
            return None
        }
        if self.data.get(w as usize).unwrap() & flag == 0 {
            return None
        }

        if self.bit_width == 0 {
            return Some(0)
        }

        let c = &(&(self.data))[(w + u8::BITS / u8::BITS) as usize..(w + block_step) as usize];
        let w_v = b * (self.bit_width as u32 / u8::BITS) / u8::BITS;
        let b_v = b * (self.bit_width as u32 / u8::BITS) % u8::BITS;
        let v = c[w_v as usize];
        let flag = ((2 as u8).pow(self.bit_width as u32 / u8::BITS) as u8 - 1) << b_v;
        Some(((v & flag) >> b_v) as u8)
    }

    pub fn set_val(&mut self, index: u8, val: u8) {
        let block_step = u8::BITS / u8::BITS + self.bit_width as u32 / u8::BITS;
        let w = index as u32 / u8::BITS * block_step;
        let b = index as u32 % u8::BITS;
        for i in 0..w + 1 * block_step {
            if self.data.get(i as usize).is_none() {
                self.data.push(0);
            }
        }

        let flag = (1 as u8) << b;
        let k = (&(self.data))[w as usize] | flag;
        let e = self.data.get_mut(w as usize).unwrap();
        *e = k;
        
        if self.bit_width == 0 {
            return
        }

        let c = &(&(self.data))[(w + u8::BITS / u8::BITS) as usize..(w + block_step) as usize];
        let w_v = b * (self.bit_width as u32 / u8::BITS) / u8::BITS;
        let b_v = b * (self.bit_width as u32 / u8::BITS) % u8::BITS;
        let mut v = c[w_v as usize];
        let flag = ((2 as u8).pow(self.bit_width as u32 / u8::BITS) as u8 - 1) << b_v;
        v = v & !flag;
        let flag = ((val as u8) as u8) << b_v;
        v = v | flag;
        let e = self.data.get_mut((w + (u8::BITS / u8::BITS) + w_v) as usize).unwrap();
        *e = v;
    }

    pub fn del_val(&mut self, index: u8) {
        let block_step = u8::BITS / u8::BITS + self.bit_width as u32 / u8::BITS;
        let w = index as u32 / u8::BITS * block_step;
        let b = index as u32 % u8::BITS;
        let flag = (1 as u8) << b;
        
        if self.data.get((w + block_step - 1) as usize).is_none() {
            return
        }
        let k = (&(self.data))[w as usize] & !flag;
        let e = self.data.get_mut(w as usize).unwrap();
        *e = k;

        if self.bit_width == 0 {
            return
        }

        let c = &(&(self.data))[(w + u8::BITS / u8::BITS) as usize..(w + block_step) as usize];
        let w_v = b * (self.bit_width as u32 / u8::BITS) / u8::BITS;
        let b_v = b * (self.bit_width as u32 / u8::BITS) % u8::BITS;
        let mut v = c[w_v as usize];
        let flag = ((2 as u8).pow(self.bit_width as u32 / u8::BITS) as u8 - 1) << b_v;
        v = v & !flag;
        let e = self.data.get_mut((w + (u8::BITS / u8::BITS) + w_v) as usize).unwrap();
        *e = v;
    }
}

impl TryInto<Vec<u8>> for RawValue {
    type Error = ();

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.data)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::RawValue;


    #[test]
    pub fn test() {
        let mut raw_value = RawValue::new(Vec::new(), 2);
        raw_value.set_val(99, 1);
        let res = raw_value.del_val(99);
    }

}
