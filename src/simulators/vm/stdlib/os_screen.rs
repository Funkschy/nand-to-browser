use super::*;
use crate::definitions::{Address, Word, SCREEN_END, SCREEN_HEIGHT, SCREEN_START, SCREEN_WIDTH};

use lazy_static::lazy_static;
use std::sync::Mutex;

// at the point of writing this, wasm has no atomics support, so we have to use a mutex instead
lazy_static! {
    static ref BLACK: Mutex<bool> = Mutex::new(true);
}

fn is_black() -> Result<bool, StdlibError> {
    BLACK
        .lock()
        .map_err(|_| StdlibError::ScreenBlockedColorMutex)
        .map(|b| *b)
}

fn set_black(value: bool) -> Result<(), StdlibError> {
    set_mutex!(BLACK, value, StdlibError::ScreenBlockedColorMutex);
    Ok(())
}

pub fn init<VM: VirtualMachine>(_vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    set_black(true)?;
    Ok(StdlibOk::Finished(0))
}

pub fn clear_screen<VM: VirtualMachine>(vm: &mut VM, _: State, _params: &[Word]) -> StdResult {
    for i in SCREEN_START..=SCREEN_END {
        vm.set_mem(i, 0);
    }
    Ok(StdlibOk::Finished(0))
}

pub fn set_color<VM: VirtualMachine>(_vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    set_black(params[0] != 0)?;
    Ok(StdlibOk::Finished(0))
}

fn update_location<VM: VirtualMachine>(
    vm: &mut VM,
    address: Address,
    mask: u16,
) -> Result<(), StdlibError> {
    let address = address + SCREEN_START;
    let mut value = vm.mem(address) as u16;

    if is_black()? {
        value |= mask;
    } else {
        value &= !mask;
    }

    vm.set_mem(address, value as Word);
    Ok(())
}

fn check_bounds(x: Word, y: Word) -> Result<(), StdlibError> {
    if x < 0 || x >= SCREEN_WIDTH as i16 || y < 0 || y >= SCREEN_HEIGHT as i16 {
        Err(StdlibError::ScreenIllegalCoords)
    } else {
        Ok(())
    }
}

pub fn draw_pixel<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let x = params[0];
    let y = params[1];

    check_bounds(x, y)?;

    let address = (y as usize * SCREEN_WIDTH + x as usize) >> 4;
    let mask = 1 << (x & 15);
    update_location(vm, address, mask)?;

    Ok(StdlibOk::Finished(0))
}

fn draw_conditional<VM: VirtualMachine>(
    vm: &mut VM,
    x: Word,
    y: Word,
    exchange: bool,
) -> Result<(), StdlibError> {
    let (a, b) = if exchange {
        // upcast to handle overflows while multiplying
        (y as i32, x as i32)
    } else {
        (x as i32, y as i32)
    };
    let address = (b * SCREEN_WIDTH as i32 + a) >> 4;
    let mask = 1 << (a & 15);
    update_location(vm, address as usize, mask)
}

pub fn draw_line<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let mut x1 = params[0];
    let mut y1 = params[1];
    let mut x2 = params[2];
    let mut y2 = params[3];

    check_bounds(x1, y1)?;
    check_bounds(x2, y2)?;

    let mut dx = (x2 - x1).abs();
    let mut dy = (y2 - y1).abs();

    let loop_over_y = dx < dy;
    if loop_over_y && (y2 < y1) || !loop_over_y && (x2 < x1) {
        std::mem::swap(&mut x1, &mut x2);
        std::mem::swap(&mut y1, &mut y2);
    }

    let (x, y, end_x, delta_y) = if loop_over_y {
        std::mem::swap(&mut dx, &mut dy);
        let delta = if x1 > x2 { -1 } else { 1 };
        (y1, x1, y2, delta)
    } else {
        let delta = if y1 > y2 { -1 } else { 1 };
        (x1, y1, x2, delta)
    };

    draw_conditional(vm, x, y, loop_over_y)?;
    // var = 2*x*dy - 2*(|y|-0.5)*dx
    // ==> var >=0 iff 2*x*dy >= 2*(|y|-0.5)*dx
    // iff dy/dx >= x/(|y|-0.5)
    let mut var = 2 * dy - dx;
    let two_y = 2 * dy;
    let two_y_minus_two_dx = two_y - 2 * dx;

    let mut y = y;
    for x in x..end_x {
        if var < 0 {
            var += two_y;
        } else {
            var += two_y_minus_two_dx;
            y += delta_y;
        }
        draw_conditional(vm, x + 1, y, loop_over_y)?;
    }

    Ok(StdlibOk::Finished(0))
}

pub fn draw_rectangle<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let x1 = params[0];
    let y1 = params[1];
    let x2 = params[2];
    let y2 = params[3];
    check_bounds(x1, y1)?;
    check_bounds(x2, y2)?;
    let x1_word = x1 >> 4; // x1 / 16
    let x2_word = x2 >> 4;

    let mut address = (y1 as usize * (SCREEN_WIDTH >> 4)) + x1_word as usize;
    let first_mask = 0xFFFF << (x1 & 15);
    let last_mask = 0xFFFF >> (15 - (x2 & 15));
    let mask = last_mask & first_mask;

    let diff = (x2_word - x1_word) as usize;

    if diff == 0 {
        for _ in y1..=y2 {
            update_location(vm, address, mask)?;
            address += SCREEN_WIDTH >> 4;
        }
    } else {
        for _ in y1..=y2 {
            let last_address_in_line = address + diff;
            update_location(vm, address, first_mask)?;
            address += 1;
            let start = address; // don't modify loop range, because clippy does not like that
            for _ in start..last_address_in_line {
                update_location(vm, address, 0xFFFF)?;
                address += 1;
            }
            update_location(vm, address, last_mask)?;
            address += (SCREEN_WIDTH >> 4) - diff;
        }
    }

    Ok(StdlibOk::Finished(0))
}

fn draw_two_horizontal<VM: VirtualMachine>(
    vm: &mut VM,
    y1: Word,
    y2: Word,
    min_x: Word,
    max_x: Word,
) -> Result<(), StdlibError> {
    let min_x_word = min_x >> 4;
    let max_x_word = max_x >> 4;

    let mut address1 = (y1 as usize * (SCREEN_WIDTH >> 4)) + min_x_word as usize;
    let mut address2 = (y2 as usize * (SCREEN_WIDTH >> 4)) + min_x_word as usize;

    let first_mask = 0xFFFF << (min_x & 15);
    let last_mask = 0xFFFF >> (15 - (max_x & 15));
    let mask = last_mask & first_mask;

    let diff = (max_x_word - min_x_word) as usize;
    if diff == 0 {
        update_location(vm, address1, mask)?;
        update_location(vm, address2, mask)?;
    } else {
        let last_address_in_line = address1 + diff;
        update_location(vm, address1, first_mask)?;
        update_location(vm, address2, first_mask)?;

        address1 += 1;
        address2 += 1;

        let start = address1; // don't modify loop range, because clippy does not like that
        for _ in start..last_address_in_line {
            update_location(vm, address1, 0xFFFF)?;
            update_location(vm, address2, 0xFFFF)?;
            address1 += 1;
            address2 += 1;
        }

        update_location(vm, address1, last_mask)?;
        update_location(vm, address2, last_mask)?;
    }

    Ok(())
}

pub fn draw_circle<VM: VirtualMachine>(vm: &mut VM, _: State, params: &[Word]) -> StdResult {
    let x = params[0];
    let y = params[1];
    let r = params[2];

    check_bounds(x, y)?;
    check_bounds(x - r, y - r)?;
    check_bounds(x + r, y + r)?;

    let mut delta1 = 0;
    let mut delta2 = r;
    let mut var = 1 - r;

    draw_two_horizontal(vm, y - delta2, y + delta2, x - delta1, x + delta1)?;
    draw_two_horizontal(vm, y - delta1, y + delta1, x - delta2, x + delta2)?;

    while delta2 > delta1 {
        if var < 0 {
            var += 2 * delta1 + 3;
        } else {
            var += 2 * (delta1 - delta2) + 5;
            delta2 -= 1;
        }

        delta1 += 1;

        draw_two_horizontal(vm, y - delta2, y + delta2, x - delta1, x + delta1)?;
        draw_two_horizontal(vm, y - delta1, y + delta1, x - delta2, x + delta2)?;
    }

    Ok(StdlibOk::Finished(0))
}
