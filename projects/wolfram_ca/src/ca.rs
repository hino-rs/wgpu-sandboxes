pub struct Ca {
    pub num_of_bits: u16,
    pub cells: Vec<u8>,
}

impl Ca {
    pub fn append_next(&mut self) {
        let current_num_of_bits = self.num_of_bits as usize;
        let current_last_bits = &self.cells.clone()[self.cells.len() - current_num_of_bits..];
        
        let max_rows = current_num_of_bits;
        if self.cells.len() > max_rows * current_num_of_bits {
            self.cells.drain(0..current_num_of_bits);
        }

        let mut next_bits = Vec::with_capacity(current_num_of_bits);
        // Next = Left XOR (Center OR Right)

        for i in 0..current_num_of_bits {
            let center = current_last_bits[i];
            let next = if i == 0 || i == current_num_of_bits-1 {
                false
            } else {
                let tmp = center == 1 || current_last_bits[i+1] == 1;
                current_last_bits[i-1] != tmp as u8
            };
            next_bits.push(next as u8);
        }

        self.cells.append(&mut next_bits);
    }
}
