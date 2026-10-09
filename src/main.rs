#[derive(Debug, thiserror::Error)]
pub enum HDError {
    #[error["Address is out of bound"]]
    OutOfBound,
    #[error["Addition overflows"]]
    Overflow,
}

#[repr(u8)]
enum LCDType {
    A00 = 0,
    A02 = 1,
    UserDefined = 2
}

pub const DDRAMSIZE: usize = 80;
pub const CGRAMSIZE: usize = 64;

struct Hd44780 {
    // Internal    
    ddram: [u8; DDRAMSIZE],
    cgram: [u8; CGRAMSIZE],
    busy_flag: bool,
    address_counter: usize,

    // Physical I/O
    data_register: u8,
    rs: bool,
    rw: bool,
    e: bool,
    com: u16,
    seg: u64,

    // Control
    n: bool,
    i_d: bool,
    s: bool,
    display: bool,
    cursor: bool, 
    blink: bool,
    sc: bool,
    rl: bool,
    dl: bool,
    f: bool,
    lcd_type: LCDType,
    backlight: bool,
    contrast: f64,    
    dirty: bool,
}

impl Default for Hd44780 {
    fn default() -> Self {
        Self {
            ddram: [0; DDRAMSIZE],
            cgram: [0; CGRAMSIZE],
            busy_flag: false,
            address_counter: 0,

            data_register: 0,
            rs: false,
            rw: false,
            e: false,
            com: 0,
            seg: 0,

            n: true,
            i_d: true,
            s: false,
            display: true,
            cursor: true, 
            blink: true,
            sc: false,
            rl: true,
            dl: true,
            f: false,
            lcd_type: LCDType::A02,
            backlight: true,
            contrast: 0.5,
            dirty: false 
        }
    }
}

impl Hd44780 {
    fn new(lcd_type: LCDType) -> Self {
        Self{
            lcd_type: lcd_type,
            ..Default::default()
        }
    }
}

#[repr(u8)]
enum LCDControllerType {
    DirectCommand = 0,
    Parallel = 1,
    I2C = 3    
}

#[repr(u8)]
enum LCDCommand {
    None = 0,
    Clear = 1,
    ReturnHome = 2,
    EntryModeSet = 3,
    DisplayControl = 4,
    CursorShift = 5,
    FunctionSet = 6,
    CGRAMAddress = 7,
    DDRAMAddress = 8,
    Data = 9
}

fn rotate_array_right<const N: usize>(arr: &mut [u8; N], p: usize) -> Result<(), HDError> {
    if p < 1 || p > arr.len() {
        return Err(HDError::OutOfBound);
    }

    arr[..p-1].reverse();
    arr[..p].reverse();
    Ok(())
}

fn rotate_array_left<const N: usize>(arr: &mut [u8; N], p: usize) -> Result<(), HDError> {
    if p < 1 || p > arr.len() {
        return Err(HDError::OutOfBound);
    }

    arr[1..p].reverse();
    arr[..p].reverse();
    Ok(())
}
    
#[repr(u8)]
#[derive(PartialEq)]
enum LCDMemoryMode {
    None = 0,
    CGRAM = 1,
    DDRAM = 2
}

struct Hd44780Controller {
    model: Hd44780,
    controller_type: LCDControllerType,

    last_enable: bool,
    data_buffer: u8,
    memory_mode: LCDMemoryMode,
}

impl Hd44780Controller {
    fn new(lcd_type: LCDType, controller_type: LCDControllerType) -> Self {
        Self{
            model: Hd44780::new(lcd_type),
            controller_type: controller_type,
            last_enable: false,
            data_buffer: 0x0,
            memory_mode: LCDMemoryMode::None
        }
    }

    pub fn set_cgram(&mut self, address: usize, value: u8) -> Result<(), HDError> {
        if address >= CGRAMSIZE {
            return Err(HDError::OutOfBound);
        }

        self.model.cgram[address]  = value;
        Ok(())
    }
    
    pub fn set_ddram(&mut self, address: usize, value: u8) -> Result<(), HDError> {
        if address >= DDRAMSIZE {
            return Err(HDError::OutOfBound);
        }

        self.model.ddram[address]  = value;
        Ok(())
    }

    pub fn get_cgram(&self, address: usize) -> Result<u8, HDError> {
        if address >= CGRAMSIZE {
            return Err(HDError::OutOfBound);
        }

        Ok(self.model.cgram[address])
    }
    
    pub fn get_ddram(&self, address: usize) -> Result<u8, HDError> {
        if address >= DDRAMSIZE {
            return Err(HDError::OutOfBound);
        }

        Ok(self.model.ddram[address])
    }

    pub fn decode(&mut self) -> Result<(LCDCommand, u8), HDError> {
        let mut command: LCDCommand = LCDCommand::None;
        let mut value  : u8         = 0;

        if self.model.rs {
            command = LCDCommand::Data;
            value = self.data_buffer;
            return Ok((command, value));
        }
        
        if (self.data_buffer & 0x80) != 0x0 {
            command = LCDCommand::DDRAMAddress;
            value   = self.data_buffer & 0x7F;
            return Ok((command, value));
        }
        
        if (self.data_buffer & 0x40) != 0x0 {
            command = LCDCommand::CGRAMAddress;
            value   = self.data_buffer & 0x3F;
            return Ok((command, value));
        }
        
        if (self.data_buffer & 0x20) != 0x0 {
            command = LCDCommand::FunctionSet;
            value   = self.data_buffer & 0x1C;
            return Ok((command, value));
        }
        
        if (self.data_buffer & 0x10) != 0x0 {
            command = LCDCommand::CursorShift;
            value   = self.data_buffer & 0xC;
            return Ok((command, value));
        }

        if (self.data_buffer & 0x08) != 0x0 {
            command = LCDCommand::DisplayControl;
            value   = self.data_buffer & 0x7;
            return Ok((command, value));
        }
        
        if (self.data_buffer & 0x04) != 0x0 {
            command = LCDCommand::EntryModeSet;
            value   = self.data_buffer & 0x3;
            return Ok((command, value));
        }
        
        if (self.data_buffer & 0x02) != 0x0 {
            command = LCDCommand::ReturnHome;
            return Ok((command, value));
        }
        
        if (self.data_buffer & 0x01) != 0x0 {
            command = LCDCommand::Clear;
            return Ok((command, value));
        }

        // None, 0
        return Ok((command, value));
    }

    pub fn clear_ddram(&mut self) -> Result<(), HDError> {
        for n in self.model.ddram.iter_mut() {
            *n = 0x0;
        }
        Ok(())
    }

    pub fn handle_clear(&mut self, value: u8) -> Result<(), HDError> {
        let _ = self.clear_ddram()?;

        self.model.i_d = true;     
        Ok(())
    }

    pub fn handle_return_home(&mut self, value: u8) -> Result<(), HDError> {
        self.memory_mode = LCDMemoryMode::DDRAM;
        self.model.address_counter = 0;
        Ok(())
    }
    
    pub fn handle_entry_mode_set(&mut self, value: u8) -> Result<(), HDError> {
        self.model.s   = (value & 0x1) != 0;
        self.model.i_d = (value & 0x2) != 0;
        Ok(())
    }
    
    pub fn handle_display_control(&mut self, value: u8) -> Result<(), HDError> {
        self.model.display = (value & 0x4) != 0;
        self.model.cursor  = (value & 0x2) != 0;
        self.model.blink   = (value & 0x1) != 0;
        Ok(())
    }
    
    pub fn handle_cursor_shift(&mut self, value: u8) -> Result<(), HDError> {
        self.model.sc   = (value & 0x4) != 0;
        self.model.rl   = (value & 0x8) != 0;
        Ok(())
    }
    
    pub fn handle_function_set(&mut self, value: u8) -> Result<(), HDError> {
        self.model.f    = (value & 0x04) != 0;
        self.model.n    = (value & 0x08) != 0;
        self.model.dl   = (value & 0x10) != 0;
        Ok(())
    }
    
    pub fn handle_cgram_address(&mut self, value: u8) -> Result<(), HDError> {
        self.memory_mode      = LCDMemoryMode::CGRAM;
        self.model.address_counter = value as usize;
        Ok(())
    }
    
    pub fn handle_ddram_address(&mut self, value: u8) -> Result<(), HDError> {
        self.memory_mode     = LCDMemoryMode::DDRAM;
        self.model.address_counter = value as usize;
        Ok(())
    }

    pub fn rotate_ddram(&mut self, l_r: bool, p: usize) -> Result<(), HDError>{
        if l_r {
            let _ = rotate_array_left(&mut self.model.ddram, p)?;
        } else {
            let _ = rotate_array_right(&mut self.model.ddram, p)?;
        }
        
        Ok(())
    }

    pub fn increment_address_counter(&mut self) -> Result<(), HDError> {
        let mut new_ac: usize = self.model.address_counter.checked_add(1).ok_or(HDError::Overflow)?;


        if (self.memory_mode == LCDMemoryMode::DDRAM) && (new_ac >= DDRAMSIZE) {
            new_ac = DDRAMSIZE - 1;
        } else if (self.memory_mode == LCDMemoryMode::CGRAM) && (new_ac >= CGRAMSIZE) {
            new_ac = CGRAMSIZE - 1;
        }

        self.model.address_counter = new_ac;
        Ok(())
    }

    pub fn decrement_address_counter(&mut self) -> Result<(), HDError> {
        let mut new_ac: usize = self.model.address_counter;
        
        if new_ac > 0 {
            new_ac = new_ac - 1;
        } else {
            new_ac = 0;
        }
        
        self.model.address_counter = new_ac;
        Ok(())
    }
    
    pub fn putc(&mut self, c: u8) -> Result<(), HDError> {
        if self.model.address_counter == 0 {
            _ = self.set_ddram(0, c)?;

            if !self.model.s {
                _ = self.increment_address_counter()?;
            }

            return Ok(());
        }
        
        if self.model.s && !self.model.i_d {
            // Shift, and move right
            _ = rotate_array_right(&mut self.model.ddram, self.model.address_counter)?;
            _ = self.set_ddram(0, c)?;
        } else if self.model.s && self.model.i_d {
            // Shift, and move left
            _ = rotate_array_left(&mut self.model.ddram, self.model.address_counter)?;
            _ = self.set_ddram(self.model.address_counter - 1, c)?;
        } else if !self.model.s && !self.model.i_d {
            // Move cursor, and insert at the start
            _ = rotate_array_right(&mut self.model.ddram, self.model.address_counter)?;
            _ = self.set_ddram(0, c)?;
            _ = self.increment_address_counter()?;
        } else {
            _ = self.set_ddram(self.model.address_counter, c)?;
            _ = self.increment_address_counter()?;
            
        }

        Ok(())
    }
    
    pub fn handle_data_write(&mut self, value: u8) -> Result<(), HDError> {
        
        // Handle write
        if self.memory_mode == LCDMemoryMode::CGRAM {
            let _ = self.set_cgram(self.model.address_counter, value)?;
        } else if self.memory_mode == LCDMemoryMode::DDRAM {
            let _ = self.set_ddram(self.model.address_counter, value)?;
        }

        // Handle address advance or rewind
        
           
        Ok(())
    }

    pub fn watch_ddram(&mut self) -> Result<(), HDError> {
        for (i,n) in self.model.ddram.iter().enumerate() {
            print!("{:02x} ", n);

            if (i & 0xF) == 0xF {
                print!("\n");
            }
        }
    
        print!("\n");
    
        for (i,n) in self.model.ddram.iter().enumerate() {
            print!("{}  ", *n as char);
            
            if (i & 0xF) == 0xF {
                print!("\n");
            }
        }
    
        print!("\n");
        Ok(())
    }
}

fn main() {
    let mut lcd: Hd44780Controller = Hd44780Controller::new(LCDType::A02, LCDControllerType::DirectCommand);

    // Small test
    lcd.putc('A' as u8);
    lcd.putc('B' as u8);
    lcd.putc('C' as u8);
    lcd.putc('D' as u8);
    lcd.putc('E' as u8);
    lcd.putc('F' as u8);
    lcd.putc('G' as u8);
    lcd.putc('H' as u8);

    lcd.watch_ddram();

    lcd.model.s = true;
    lcd.model.i_d = false;
    
    lcd.putc('I' as u8);
    lcd.putc('J' as u8);
    lcd.putc('K' as u8);
    
    lcd.watch_ddram();
    
    lcd.model.s = true;
    lcd.model.i_d = true;
    
    lcd.putc('L' as u8);
    lcd.putc('M' as u8);
    lcd.putc('N' as u8);
    
    lcd.watch_ddram();
    
    lcd.model.s = false;
    lcd.model.i_d = false;
    
    lcd.putc('O' as u8);
    lcd.putc('P' as u8);
    lcd.putc('Q' as u8);
    
    lcd.watch_ddram();
}
