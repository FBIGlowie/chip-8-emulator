use crate::Chip8;
use crate::HEIGHT;
use crate::WIDTH;

/// The memory used for the screen. Each value is
/// a boolean and represents a 1 for white, and 0 for black.
///
/// The 0th memory location maps to the top left corner
/// of the screen.
/// A memory location is given by `location = WIDTH*y + x`.
#[derive(Debug)]
pub struct Screen([u8; (WIDTH * HEIGHT) as usize]);

impl Default for Screen {
    /// Initializes screen to black.
    fn default() -> Self {
        Self([0; (WIDTH * HEIGHT) as usize])
    }
}

impl Screen {
    /// Clears the screen.
    pub fn clear(&mut self) {
        for b in self.0.iter_mut() {
            *b = 0x00;
        }
    }

    /// Inverts a pixel at a given x and y.
    ///
    /// Returns the new value of the pixel (1 for white and
    /// 0 for black). This is important as we change the value
    /// of VF to 1 if we turned a pixel off that used to be on.
    pub fn invert(&mut self, x: u8, y: u8) -> bool {
        let address = (y as usize * WIDTH as usize) + x as usize;

        let new_state = self.0[address] != 1;
        self.0[address] = new_state as u8;

        new_state
    }

    pub fn get(&self) -> &[u8; (WIDTH * HEIGHT) as usize] {
        &self.0
    }
}