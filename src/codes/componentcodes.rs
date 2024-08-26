// utility components
pub const GIVE_VELOCITY: u64 = 0; // give_velocity(x: float, y: float, z: float)
pub const TAKE_FORM: u64 = 1; // take_form(form: integer )
pub const UNDO_FORM: u64 = 2; // undo_form()
pub const RECHARGE_TO: u64 = 3; // recharge_to(energy: float)
pub const ANCHOR: u64 = 4; // anchor()
pub const UNDO_ANCHOR: u64 = 5; // undo_anchor()
pub const PERISH: u64 = 6; // perish()
pub const TAKE_SHAPE: u64 = 7; // take_shape(shape: integer)
pub const UNDO_SHAPE: u64 = 8; // undo_shape()

// logic components
pub const MOVING: u64 = 1000; // moving() returns boolean
pub const GET_TIME: u64 = 1001; // get_time() returns float

// power components
pub const SET_DAMAGE: u64 = 2000; // set_damage()
