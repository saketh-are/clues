use serde::{Deserialize, Serialize};

pub const TOUCHING_NEIGHBOR_OFFSETS: [Offset; 8] = [
    Offset::new(-1, -1),
    Offset::new(-1, 0),
    Offset::new(-1, 1),
    Offset::new(0, -1),
    Offset::new(0, 1),
    Offset::new(1, -1),
    Offset::new(1, 0),
    Offset::new(1, 1),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoardShape {
    pub rows: u8,
    pub cols: u8,
}

impl BoardShape {
    pub const fn new(rows: u8, cols: u8) -> Self {
        Self { rows, cols }
    }

    pub fn contains(self, position: Position) -> bool {
        position.row >= 0
            && position.col >= 0
            && position.row < self.rows as i16
            && position.col < self.cols as i16
    }

    pub fn touching_neighbors(self, origin: Position) -> Vec<Position> {
        TOUCHING_NEIGHBOR_OFFSETS
            .iter()
            .map(|offset| origin.shifted(*offset))
            .filter(|position| self.contains(*position))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub row: i16,
    pub col: i16,
}

impl Position {
    pub const fn new(row: i16, col: i16) -> Self {
        Self { row, col }
    }

    pub fn shifted(self, offset: Offset) -> Self {
        Self::new(
            self.row + offset.row as i16,
            self.col + offset.col as i16,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Offset {
    pub row: i8,
    pub col: i8,
}

impl Offset {
    pub const fn new(row: i8, col: i8) -> Self {
        Self { row, col }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoardShape, Position};

    #[test]
    fn center_cell_has_eight_touching_neighbors() {
        let center = Position::new(2, 1);
        let board = BoardShape::new(5, 4);

        assert_eq!(board.touching_neighbors(center).len(), 8);
    }

    #[test]
    fn corner_cell_has_three_touching_neighbors() {
        let top_left = Position::new(0, 0);
        let board = BoardShape::new(5, 4);

        assert_eq!(board.touching_neighbors(top_left).len(), 3);
    }
}
