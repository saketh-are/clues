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

    pub fn is_edge(self, position: Position) -> bool {
        position.row == 0
            || position.col == 0
            || position.row == self.rows as i16 - 1
            || position.col == self.cols as i16 - 1
    }

    pub fn is_corner(self, position: Position) -> bool {
        let top_or_bottom = position.row == 0 || position.row == self.rows as i16 - 1;
        let left_or_right = position.col == 0 || position.col == self.cols as i16 - 1;

        top_or_bottom && left_or_right
    }

    pub fn tiles_in_direction(self, origin: Position, offset: Offset) -> Vec<Position> {
        let mut tiles = Vec::new();
        let mut current = origin.shifted(offset);

        while self.contains(current) {
            tiles.push(current);
            current = current.shifted(offset);
        }

        tiles
    }

    pub fn row_positions(self, row: u8) -> Vec<Position> {
        (0..self.cols)
            .map(|col| Position::new(row as i16, col as i16))
            .collect()
    }

    pub fn col_positions(self, col: u8) -> Vec<Position> {
        (0..self.rows)
            .map(|row| Position::new(row as i16, col as i16))
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
    use super::{BoardShape, Offset, Position};

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

    #[test]
    fn top_left_has_four_tiles_below() {
        let top_left = Position::new(0, 0);
        let board = BoardShape::new(5, 4);

        assert_eq!(
            board.tiles_in_direction(top_left, Offset::new(1, 0)),
            vec![
                Position::new(1, 0),
                Position::new(2, 0),
                Position::new(3, 0),
                Position::new(4, 0),
            ],
        );
    }

    #[test]
    fn top_left_has_three_tiles_to_the_right() {
        let top_left = Position::new(0, 0);
        let board = BoardShape::new(5, 4);

        assert_eq!(
            board.tiles_in_direction(top_left, Offset::new(0, 1)),
            vec![
                Position::new(0, 1),
                Position::new(0, 2),
                Position::new(0, 3),
            ],
        );
    }

    #[test]
    fn top_left_has_no_tiles_above() {
        let top_left = Position::new(0, 0);
        let board = BoardShape::new(5, 4);

        assert!(board
            .tiles_in_direction(top_left, Offset::new(-1, 0))
            .is_empty());
    }

    #[test]
    fn top_left_has_no_tiles_to_the_left() {
        let top_left = Position::new(0, 0);
        let board = BoardShape::new(5, 4);

        assert!(board
            .tiles_in_direction(top_left, Offset::new(0, -1))
            .is_empty());
    }

    #[test]
    fn row_two_has_four_tiles() {
        let board = BoardShape::new(5, 4);

        assert_eq!(board.row_positions(2).len(), 4);
    }

    #[test]
    fn col_one_has_five_tiles() {
        let board = BoardShape::new(5, 4);

        assert_eq!(board.col_positions(1).len(), 5);
    }

    #[test]
    fn row_two_has_two_tiles_on_the_edges() {
        let board = BoardShape::new(5, 4);

        assert_eq!(
            board
                .row_positions(2)
                .into_iter()
                .filter(|position| board.is_edge(*position))
                .count(),
            2,
        );
    }

    #[test]
    fn center_cell_has_three_edge_neighbors() {
        let board = BoardShape::new(5, 4);
        let center = Position::new(2, 1);

        assert_eq!(
            board
                .touching_neighbors(center)
                .into_iter()
                .filter(|position| board.is_edge(*position))
                .count(),
            3,
        );
    }

    #[test]
    fn top_row_has_two_corner_tiles() {
        let board = BoardShape::new(5, 4);

        assert_eq!(
            board
                .row_positions(0)
                .into_iter()
                .filter(|position| board.is_corner(*position))
                .count(),
            2,
        );
    }

    #[test]
    fn inner_cell_has_one_corner_neighbor() {
        let board = BoardShape::new(5, 4);
        let inner = Position::new(1, 1);

        assert_eq!(
            board
                .touching_neighbors(inner)
                .into_iter()
                .filter(|position| board.is_corner(*position))
                .count(),
            1,
        );
    }
}
