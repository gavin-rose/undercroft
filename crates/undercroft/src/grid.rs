use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    /// Orthogonal neighbours in a fixed order (up, right, down, left).
    pub fn neighbors4(self) -> [Pos; 4] {
        let Pos { x, y } = self;
        [
            Pos { x, y: y - 1 },
            Pos { x: x + 1, y },
            Pos { x, y: y + 1 },
            Pos { x: x - 1, y },
        ]
    }

    pub fn manhattan(self, other: Pos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn center(self) -> Pos {
        Pos {
            x: self.x + self.w / 2,
            y: self.y + self.h / 2,
        }
    }
}

/// A rectangular layer of cells, stored row-major.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Grid<T> {
    width: i32,
    height: i32,
    cells: Vec<T>,
}

impl<T: Copy> Grid<T> {
    pub fn new(width: i32, height: i32, fill: T) -> Self {
        Grid {
            width,
            height,
            cells: vec![fill; (width * height) as usize],
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn index(&self, p: Pos) -> Option<usize> {
        let inside = p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height;
        inside.then(|| (p.y * self.width + p.x) as usize)
    }

    pub fn pos(&self, index: usize) -> Pos {
        let i = index as i32;
        Pos {
            x: i % self.width,
            y: i / self.width,
        }
    }

    pub fn get(&self, p: Pos) -> Option<T> {
        self.index(p).map(|i| self.cells[i])
    }

    /// Out-of-bounds writes are ignored.
    pub fn set(&mut self, p: Pos, value: T) {
        if let Some(i) = self.index(p) {
            self.cells[i] = value;
        }
    }

    pub fn cells(&self) -> &[T] {
        &self.cells
    }

    pub fn cells_mut(&mut self) -> &mut [T] {
        &mut self.cells
    }

    /// Every position in row-major order, matching [`Grid::index`].
    pub fn positions(&self) -> impl Iterator<Item = Pos> + use<T> {
        let (w, h) = (self.width, self.height);
        (0..h).flat_map(move |y| (0..w).map(move |x| Pos { x, y }))
    }
}
