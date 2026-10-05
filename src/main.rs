#[repr(u8)]
enum LCDType {
    A00 = 0,
    A02 = 1,
    UserDefined = 2
}

#[repr(u8)]
enum LCDLine {
    Single5x8 = 0,
    Single5x10 = 1,
    Double5x8 = 2
}

pub const DDRAMSIZE: usize = 80;
pub const CGRAMSIZE: usize = 64;

struct Hd44780 {
    // Internal    
    ddram: [u8; DDRAMSIZE],
    cgram: [u8; CGRAMSIZE],
    busy_flag: bool,
    address_counter: u8,

    // Physical I/O
    data_register: u8,
    rs: bool,
    rw: bool,
    e: bool,
    com: u16,
    seg: u64,

    // Control
    n: LCDLine, 
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

            n: LCDLine::Single5x8,
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

struct Hd44780Controller {
    model: Hd44780,
    controller_type: LCDControllerType,
}

impl Hd44780Controller {
    fn new(lcd_type: LCDType, controller_type: LCDControllerType) -> Self {
        Self{
            model: Hd44780::new(lcd_type),
            controller_type: controller_type
        }
    }
}

fn main() {
    let mut lcd: Hd44780Controller = Hd44780Controller::new(LCDType::A02, LCDControllerType::DirectCommand);
}
