use serde::{Deserialize, Serialize};

pub const TOUCHING_NEIGHBOR_OFFSETS: [Offset; 26] = [
    Offset::new_3d(-1, -1, -1),
    Offset::new_3d(-1, -1, 0),
    Offset::new_3d(-1, -1, 1),
    Offset::new_3d(-1, 0, -1),
    Offset::new_3d(-1, 0, 0),
    Offset::new_3d(-1, 0, 1),
    Offset::new_3d(-1, 1, -1),
    Offset::new_3d(-1, 1, 0),
    Offset::new_3d(-1, 1, 1),
    Offset::new_3d(0, -1, -1),
    Offset::new_3d(0, -1, 0),
    Offset::new_3d(0, -1, 1),
    Offset::new_3d(0, 0, -1),
    Offset::new_3d(0, 0, 1),
    Offset::new_3d(0, 1, -1),
    Offset::new_3d(0, 1, 0),
    Offset::new_3d(0, 1, 1),
    Offset::new_3d(1, -1, -1),
    Offset::new_3d(1, -1, 0),
    Offset::new_3d(1, -1, 1),
    Offset::new_3d(1, 0, -1),
    Offset::new_3d(1, 0, 0),
    Offset::new_3d(1, 0, 1),
    Offset::new_3d(1, 1, -1),
    Offset::new_3d(1, 1, 0),
    Offset::new_3d(1, 1, 1),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoardShape {
    #[serde(default = "default_depth")]
    pub depth: u8,
    pub rows: u8,
    pub cols: u8,
}

const fn default_depth() -> u8 {
    1
}

impl BoardShape {
    pub const fn new(rows: u8, cols: u8) -> Self {
        Self {
            depth: 1,
            rows,
            cols,
        }
    }

    pub const fn new_3d(depth: u8, rows: u8, cols: u8) -> Self {
        Self { depth, rows, cols }
    }

    pub fn contains(self, position: Position) -> bool {
        position.layer >= 0
            && position.row >= 0
            && position.col >= 0
            && position.layer < self.depth as i16
            && position.row < self.rows as i16
            && position.col < self.cols as i16
    }

    pub fn cell_count(self) -> usize {
        self.depth as usize * self.rows as usize * self.cols as usize
    }

    pub fn all_positions(self) -> Vec<Position> {
        let mut positions = Vec::with_capacity(self.cell_count());

        for layer in 0..self.depth {
            for row in 0..self.rows {
                for col in 0..self.cols {
                    positions.push(Position::new_3d(layer as i16, row as i16, col as i16));
                }
            }
        }

        positions
    }

    pub fn touching_neighbors(self, origin: Position) -> Vec<Position> {
        let mut positions = Vec::new();

        for layer in -1..=1 {
            for row in -1..=1 {
                for col in -1..=1 {
                    if layer == 0 && row == 0 && col == 0 {
                        continue;
                    }

                    let position = origin.shifted(Offset::new_3d(layer, row, col));
                    if self.contains(position) {
                        positions.push(position);
                    }
                }
            }
        }

        positions
    }

    pub fn orthogonal_neighbors(self, origin: Position) -> Vec<Position> {
        let offsets = [
            Offset::new(-1, 0),
            Offset::new(0, -1),
            Offset::new(0, 1),
            Offset::new(1, 0),
            Offset::new_3d(-1, 0, 0),
            Offset::new_3d(1, 0, 0),
        ];

        offsets
            .into_iter()
            .map(|offset| origin.shifted(offset))
            .filter(|position| self.contains(*position))
            .collect()
    }

    pub fn is_edge(self, position: Position) -> bool {
        (self.depth > 1
            && (position.layer == 0 || position.layer == self.depth as i16 - 1))
            || (self.rows > 1 && (position.row == 0 || position.row == self.rows as i16 - 1))
            || (self.cols > 1 && (position.col == 0 || position.col == self.cols as i16 - 1))
    }

    pub fn is_corner(self, position: Position) -> bool {
        let front_or_back = position.layer == 0 || position.layer == self.depth as i16 - 1;
        let top_or_bottom = position.row == 0 || position.row == self.rows as i16 - 1;
        let left_or_right = position.col == 0 || position.col == self.cols as i16 - 1;

        front_or_back && top_or_bottom && left_or_right
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
        let mut positions = Vec::with_capacity(self.depth as usize * self.cols as usize);

        for layer in 0..self.depth {
            for col in 0..self.cols {
                positions.push(Position::new_3d(layer as i16, row as i16, col as i16));
            }
        }

        positions
    }

    pub fn col_positions(self, col: u8) -> Vec<Position> {
        let mut positions = Vec::with_capacity(self.depth as usize * self.rows as usize);

        for layer in 0..self.depth {
            for row in 0..self.rows {
                positions.push(Position::new_3d(layer as i16, row as i16, col as i16));
            }
        }

        positions
    }

    pub fn layer_positions(self, layer: u8) -> Vec<Position> {
        let mut positions = Vec::with_capacity(self.rows as usize * self.cols as usize);

        for row in 0..self.rows {
            for col in 0..self.cols {
                positions.push(Position::new_3d(layer as i16, row as i16, col as i16));
            }
        }

        positions
    }

    pub fn positions_between(self, first: Position, second: Position) -> Vec<Position> {
        let same_layer = first.layer == second.layer;
        let same_row = first.row == second.row;
        let same_col = first.col == second.col;
        let aligned_axes = same_layer as u8 + same_row as u8 + same_col as u8;

        if aligned_axes != 2 {
            return Vec::new();
        }

        if same_layer && same_row {
            let start = first.col.min(second.col) + 1;
            let end = first.col.max(second.col);

            (start..end)
                .map(|col| Position::new_3d(first.layer, first.row, col))
                .collect()
        } else if same_layer && same_col {
            let start = first.row.min(second.row) + 1;
            let end = first.row.max(second.row);

            (start..end)
                .map(|row| Position::new_3d(first.layer, row, first.col))
                .collect()
        } else {
            let start = first.layer.min(second.layer) + 1;
            let end = first.layer.max(second.layer);

            (start..end)
                .map(|layer| Position::new_3d(layer, first.row, first.col))
                .collect()
        }
    }

    pub fn common_neighbors(self, first: Position, second: Position) -> Vec<Position> {
        let second_neighbors = self.touching_neighbors(second);

        self.touching_neighbors(first)
            .into_iter()
            .filter(|position| second_neighbors.contains(position))
            .collect()
    }

    pub fn index_of(self, position: Position) -> usize {
        position.layer as usize * self.rows as usize * self.cols as usize
            + position.row as usize * self.cols as usize
            + position.col as usize
    }

    pub fn position_of_index(self, index: usize) -> Position {
        let plane_size = self.rows as usize * self.cols as usize;
        let layer = index / plane_size;
        let within_layer = index % plane_size;
        let row = within_layer / self.cols as usize;
        let col = within_layer % self.cols as usize;

        Position::new_3d(layer as i16, row as i16, col as i16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    #[serde(default)]
    pub layer: i16,
    pub row: i16,
    pub col: i16,
}

impl Position {
    pub const fn new(row: i16, col: i16) -> Self {
        Self { layer: 0, row, col }
    }

    pub const fn new_3d(layer: i16, row: i16, col: i16) -> Self {
        Self { layer, row, col }
    }

    pub fn shifted(self, offset: Offset) -> Self {
        Self::new_3d(
            self.layer + offset.layer as i16,
            self.row + offset.row as i16,
            self.col + offset.col as i16,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Offset {
    #[serde(default)]
    pub layer: i8,
    pub row: i8,
    pub col: i8,
}

impl Offset {
    pub const fn new(row: i8, col: i8) -> Self {
        Self { layer: 0, row, col }
    }

    pub const fn new_3d(layer: i8, row: i8, col: i8) -> Self {
        Self { layer, row, col }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoardShape, Offset, Position};

    #[test]
    fn center_cell_has_eight_touching_neighbors_in_2d() {
        let center = Position::new(2, 1);
        let board = BoardShape::new(5, 4);

        assert_eq!(board.touching_neighbors(center).len(), 8);
    }

    #[test]
    fn center_cell_has_four_orthogonal_neighbors_in_2d() {
        let center = Position::new(2, 1);
        let board = BoardShape::new(5, 4);

        assert_eq!(board.orthogonal_neighbors(center).len(), 4);
    }

    #[test]
    fn center_cell_has_twenty_six_touching_neighbors_in_3d() {
        let center = Position::new_3d(1, 1, 1);
        let board = BoardShape::new_3d(3, 3, 3);

        assert_eq!(board.touching_neighbors(center).len(), 26);
    }

    #[test]
    fn center_cell_has_six_orthogonal_neighbors_in_3d() {
        let center = Position::new_3d(1, 1, 1);
        let board = BoardShape::new_3d(3, 3, 3);

        assert_eq!(board.orthogonal_neighbors(center).len(), 6);
    }

    #[test]
    fn corner_cell_has_three_touching_neighbors_in_2d() {
        let top_left = Position::new(0, 0);
        let board = BoardShape::new(5, 4);

        assert_eq!(board.touching_neighbors(top_left).len(), 3);
    }

    #[test]
    fn corner_cell_has_two_orthogonal_neighbors_in_2d() {
        let top_left = Position::new(0, 0);
        let board = BoardShape::new(5, 4);

        assert_eq!(board.orthogonal_neighbors(top_left).len(), 2);
    }

    #[test]
    fn center_cell_is_not_an_edge_in_2d() {
        let board = BoardShape::new(3, 3);

        assert!(!board.is_edge(Position::new(1, 1)));
        assert!(board.is_edge(Position::new(0, 1)));
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
    fn front_top_left_has_two_tiles_behind() {
        let front_top_left = Position::new_3d(0, 0, 0);
        let board = BoardShape::new_3d(3, 4, 5);

        assert_eq!(
            board.tiles_in_direction(front_top_left, Offset::new_3d(1, 0, 0)),
            vec![Position::new_3d(1, 0, 0), Position::new_3d(2, 0, 0)],
        );
    }

    #[test]
    fn positions_between_two_cells_in_a_row_are_returned() {
        let board = BoardShape::new(5, 4);

        assert_eq!(
            board.positions_between(Position::new(1, 0), Position::new(1, 3)),
            vec![Position::new(1, 1), Position::new(1, 2)],
        );
    }

    #[test]
    fn positions_between_two_cells_across_layers_are_returned() {
        let board = BoardShape::new_3d(4, 3, 3);

        assert_eq!(
            board.positions_between(Position::new_3d(0, 1, 1), Position::new_3d(3, 1, 1)),
            vec![Position::new_3d(1, 1, 1), Position::new_3d(2, 1, 1)],
        );
    }

    #[test]
    fn adjacent_horizontal_cells_have_four_common_neighbors_in_2d() {
        let board = BoardShape::new(5, 4);

        assert_eq!(
            board
                .common_neighbors(Position::new(1, 1), Position::new(1, 2))
                .len(),
            4,
        );
    }

    #[test]
    fn index_round_trips_in_3d() {
        let board = BoardShape::new_3d(2, 3, 4);
        let position = Position::new_3d(1, 2, 3);

        assert_eq!(board.position_of_index(board.index_of(position)), position);
    }
}
