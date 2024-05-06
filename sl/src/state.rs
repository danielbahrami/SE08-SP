#[derive(Copy, Clone, Debug, PartialEq)]
pub enum State {
    NONE,
    INITIALIZING,
    ERROR,
    CLOSED,
    CLOSING,
    OPEN,
    OPENING,
}

pub fn make_red(color: &mut [u32; 3]) {
    color[0] = 255;
    color[1] = 0;
    color[2] = 0;
}

pub fn make_green(color: &mut [u32; 3]) {
    color[0] = 0;
    color[1] = 255;
    color[2] = 0;
}

pub fn make_blue(color: &mut [u32; 3]) {
    color[0] = 0;
    color[1] = 0;
    color[2] = 255;
}

pub fn make_yellow(color: &mut [u32; 3]) {
    color[0] = 255;
    color[1] = 255;
    color[2] = 0;
}

pub fn make_orange(color: &mut [u32; 3]) {
    color[0] = 255;
    color[1] = 140;
    color[2] = 0;
}

pub fn make_white(color: &mut  [u32; 3]) {
    color[0] = 255;
    color[1] = 255;
    color[2] = 255;
}

