use crate::types::{Position, TELEPORT_THRESHOLD, WORLD_BOUNDS};

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Valid,
    SpeedHack,
    Teleport,
    OutOfBounds,
}

pub fn validate_movement(
    old_pos: &Position,
    new_pos: &Position,
    _velocity: &Position,
    delta_time: f32,
    max_speed: f32,
) -> ValidationResult {
    if !is_in_bounds(new_pos, WORLD_BOUNDS) {
        return ValidationResult::OutOfBounds;
    }

    let distance = old_pos.distance_to(new_pos);

    if is_teleport(old_pos, new_pos, TELEPORT_THRESHOLD) {
        return ValidationResult::Teleport;
    }

    let max_allowed = max_speed * delta_time * 3.0;

    if distance > max_allowed {
        log::debug!(
            "Speed check: distance={:.2}, max={:.2}, dt={:.4}",
            distance,
            max_allowed,
            delta_time
        );
        return ValidationResult::SpeedHack;
    }

    ValidationResult::Valid
}

pub fn is_teleport(old_pos: &Position, new_pos: &Position, max_distance: f32) -> bool {
    old_pos.distance_to(new_pos) > max_distance
}

pub fn is_in_bounds(pos: &Position, bounds: f32) -> bool {
    pos.x.abs() <= bounds && pos.y.abs() <= bounds && pos.z.abs() <= bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MAX_SPEED;

    #[test]
    fn test_valid_movement() {
        let old_pos = Position::new(0.0, 0.0, 0.0);
        let new_pos = Position::new(1.0, 0.0, 0.0);
        let velocity = Position::new(10.0, 0.0, 0.0);

        let result = validate_movement(&old_pos, &new_pos, &velocity, 0.1, MAX_SPEED);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_speed_hack() {
        let old_pos = Position::new(0.0, 0.0, 0.0);
        let new_pos = Position::new(15.0, 0.0, 0.0);
        let velocity = Position::new(10.0, 0.0, 0.0);

        let result = validate_movement(&old_pos, &new_pos, &velocity, 0.1, MAX_SPEED);
        assert_eq!(result, ValidationResult::SpeedHack);
    }

    #[test]
    fn test_teleport() {
        let old_pos = Position::new(0.0, 0.0, 0.0);
        let new_pos = Position::new(25.0, 0.0, 0.0);

        assert!(is_teleport(&old_pos, &new_pos, TELEPORT_THRESHOLD));
    }

    #[test]
    fn test_out_of_bounds() {
        let pos = Position::new(1001.0, 0.0, 0.0);
        assert!(!is_in_bounds(&pos, WORLD_BOUNDS));
    }
}
