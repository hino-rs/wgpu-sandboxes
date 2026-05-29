pub struct Ca {
    pub rule: u8,
    pub num_of_bits: u16,
    pub cells: Vec<u8>,
    pub pause: bool,
    pub color_of_1: [f32; 3],
    pub color_of_0: [f32; 3],
    pub circulation: bool,
}

impl Ca {
    pub fn change_bits(&mut self) {
        let mut cells = vec![0; self.num_of_bits as usize];
        cells[self.num_of_bits as usize / 2] = 1;
        self.cells = cells;
    }

    pub fn append_next(&mut self) {
        if self.pause {
            return;
        }

        let current_num_of_bits = self.num_of_bits as usize;
        let current_last_bits = &self.cells.clone()[self.cells.len() - current_num_of_bits..];

        let max_rows = current_num_of_bits;
        if self.cells.len() > max_rows * current_num_of_bits {
            self.cells.drain(0..current_num_of_bits);
        }

        let mut next_bits = Vec::with_capacity(current_num_of_bits);

        for i in 0..current_num_of_bits {
            let center = current_last_bits[i];
            let mut left = 0;
            let mut right = 0;

            if self.circulation {
                left = if i != 0 {
                    current_last_bits[i - 1]
                } else {
                    current_last_bits[current_num_of_bits - 1]
                };
                right = if i != current_num_of_bits - 1 {
                    current_last_bits[i + 1]
                } else {
                    current_last_bits[0]
                };
            } else {
                left = if i != 0 { current_last_bits[i - 1] } else { 0 };
                right = if i != current_num_of_bits - 1 {
                    current_last_bits[i + 1]
                } else {
                    0
                };
            }

            let bit = left * 3 + center * 2 + right * 1;
            let next = (self.rule >> bit) & 1;

            next_bits.push(next);
        }

        // for i in 0..current_num_of_bits {
        //     let center = current_last_bits[i];
        //     let next = if i == 0 || i == current_num_of_bits-1 {
        //         false
        //     } else {
        //         let tmp = center == 1 || current_last_bits[i+1] == 1;
        //         current_last_bits[i-1] != tmp as u8
        //     };
        //     next_bits.push(next as u8);
        // }

        self.cells.append(&mut next_bits);
    }
}
