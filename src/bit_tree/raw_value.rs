use crate::*;

#[derive(BorshSerialize, BorshDeserialize)]
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct RawValue(Vec<u8>);

impl RawValue {
    pub fn get_val(&self, index: u8)-> Option<u8> {
        let w = index as u32 / u8::BITS * (u8::BITS / u8::BITS + u16::BITS / u8::BITS);
        let b = index as u32 % u8::BITS;
        let flag = (1 as u8) << b;
        if (&(self.0))[w as usize] | flag >> b == 0 {
            return None
        }

        let c = &(&(self.0))[(w + u8::BITS / u8::BITS) as usize..(w + u8::BITS / u8::BITS + u16::BITS / u8::BITS) as usize];
        let w_v = b * (u16::BITS / u8::BITS) / u8::BITS;
        let b_v = b * (u16::BITS / u8::BITS) % u8::BITS;
        let v = c[w_v as usize];
        let flag = ((2 as u8).pow(u16::BITS / u8::BITS) as u8 - 1) << b_v;
        Some(((v & flag) >> b_v) as u8)
    }

    pub fn set_val(&mut self, index: u8, val: u8) {
        let w = index as u32 / u8::BITS * (u8::BITS / u8::BITS + u16::BITS / u8::BITS);
        let b = index as u32 % u8::BITS;
        for i in 0..w + 1 * (u8::BITS / u8::BITS + u16::BITS / u8::BITS) {
            if self.0.get(i as usize).is_none() {
                self.0.push(0);
            }
        }

        let flag = (1 as u8) << b;
        let k = (&(self.0))[w as usize] | flag;
        let e = self.0.get_mut(w as usize).unwrap();
        *e = k;

        let c = &(&(self.0))[(w + u8::BITS / u8::BITS) as usize..(w + u8::BITS / u8::BITS + u16::BITS / u8::BITS) as usize];
        let w_v = b * (u16::BITS / u8::BITS) / u8::BITS;
        let b_v = b * (u16::BITS / u8::BITS) % u8::BITS;
        let mut v = c[w_v as usize];
        let flag = ((2 as u8).pow(u16::BITS / u8::BITS) as u8 - 1) << b_v;
        v = v & !flag;
        let flag = ((val as u8) as u8) << b_v;
        v = v | flag;
        let e = self.0.get_mut((w + (u8::BITS / u8::BITS) + w_v) as usize).unwrap();
        *e = v;
    }

    pub fn del_val(&mut self, index: u8) {
        let w = index as u32 / u8::BITS * (u8::BITS / u8::BITS + u16::BITS / u8::BITS);
        let b = index as u32 % u8::BITS;
        let flag = (1 as u8) << b;
        let k = (&(self.0))[w as usize] & !flag;
        let e = self.0.get_mut(w as usize).unwrap();
        *e = k;

        let c = &(&(self.0))[(w + u8::BITS / u8::BITS) as usize..(w + u8::BITS / u8::BITS + u16::BITS / u8::BITS) as usize];
        let w_v = b * (u16::BITS / u8::BITS) / u8::BITS;
        let b_v = b * (u16::BITS / u8::BITS) % u8::BITS;
        let mut v = c[w_v as usize];
        let flag = ((2 as u8).pow(u16::BITS / u8::BITS) as u8 - 1) << b_v;
        v = v & !flag;
        let e = self.0.get_mut((w + (u8::BITS / u8::BITS) + w_v) as usize).unwrap();
        *e = v;
    }
}

impl TryFrom<Vec<u8>> for RawValue {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() as u32 % ((u8::BITS + u16::BITS) / u8::BITS) != 0 {
            return Err(format!("{:?}, {:?}", value.len(), value))
        }
        Ok(RawValue(value))
    }
}

impl TryInto<Vec<u8>> for RawValue {
    type Error = ();

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.0)
    }
}