use godot::{
    classes::Sprite2D,
    engine::{Area2D, CollisionShape2D, IArea2D, InputEvent, RectangleShape2D},
    prelude::*,
};

use std::{
    sync::atomic::{self, AtomicBool},
    time::{Duration, Instant},
};

struct TicTacToeExtensionLibrary;
#[gdextension]
unsafe impl ExtensionLibrary for TicTacToeExtensionLibrary {}

#[derive(GodotClass)]
#[class(base=Camera2D, init)]
struct TicTacToeCamera {
    base: Base<Camera2D>,
}
#[godot_api]
impl ICamera2D for TicTacToeCamera {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SquareChoice {
    None,
    X,
    O,
}

static DIRTY_GRID: AtomicBool = AtomicBool::new(false);

#[derive(GodotClass)]
#[class(base=Node2D)]
struct TicTacToeGrid {
    squares: [[Gd<TicTacToeSquare>; 3]; 3],
    turn: SquareChoice,
    game_over_time: Option<Instant>,

    base: Base<Node2D>,
}
#[godot_api]
impl INode2D for TicTacToeGrid {
    fn init(base: Base<Self::Base>) -> Self {
        let mut squares = [
            [
                TicTacToeSquare::new_alloc(),
                TicTacToeSquare::new_alloc(),
                TicTacToeSquare::new_alloc(),
            ],
            [
                TicTacToeSquare::new_alloc(),
                TicTacToeSquare::new_alloc(),
                TicTacToeSquare::new_alloc(),
            ],
            [
                TicTacToeSquare::new_alloc(),
                TicTacToeSquare::new_alloc(),
                TicTacToeSquare::new_alloc(),
            ],
        ];

        for (y, row) in (-1..=1).zip(squares.iter_mut()) {
            for (x, square) in (-1..=1).zip(row.iter_mut()) {
                square.set_position(Vector2 {
                    x: (x * 16) as f32,
                    y: (y * 16) as f32,
                });
            }
        }

        Self {
            squares,
            turn: SquareChoice::None,
            game_over_time: None,
            base,
        }
    }

    fn ready(&mut self) {
        for square in self.squares.iter().flat_map(|row| row.iter().cloned()) {
            self.to_gd().add_child(square.upcast());
        }
    }

    fn process(&mut self, _delta: f64) {
        if DIRTY_GRID.swap(false, atomic::Ordering::Relaxed) {
            self.check_game_over();
        }

        if let Some(time) = self.game_over_time {
            self.check_game_reset(time);
        }
    }
}
#[godot_api]
impl TicTacToeGrid {
    fn check_game_reset(&mut self, time: Instant) {
        // Dead zone for reset is 1 second
        if time.elapsed() < Duration::from_secs(1) {
            return;
        }

        for mut square in self
            .squares
            .iter_mut()
            .flat_map(|row| row.iter_mut().map(|square| square.bind_mut()))
        {
            square.choice = SquareChoice::None;
            square.is_part_of_solution = false;
        }

        self.game_over_time = None;
        self.turn = SquareChoice::None;
    }

    fn check_game_over(&mut self) {
        let mut game_over = false;
        for row in 0..3 {
            game_over |= self.check_specific_game_over_case([(row, 0), (row, 1), (row, 2)]);
        }

        for column in 0..3 {
            game_over |=
                self.check_specific_game_over_case([(0, column), (1, column), (2, column)]);
        }

        for diagonal in [[(0, 0), (1, 1), (2, 2)], [(0, 2), (1, 1), (2, 0)]] {
            game_over |= self.check_specific_game_over_case(diagonal);
        }

        if game_over || self.all_are_filled() {
            self.game_over_time = Some(Instant::now());
        }
    }

    #[inline]
    fn all_are_filled(&self) -> bool {
        self.squares
            .iter()
            .flat_map(|row| row.iter().map(|square| square.bind().choice))
            .all(|choice| choice != SquareChoice::None)
    }

    #[inline]
    fn check_specific_game_over_case(&mut self, squares: [(usize, usize); 3]) -> bool {
        if matches!(
            self.squares[squares[0].0][squares[0].1].bind().choice,
            SquareChoice::O | SquareChoice::X
        ) && squares.windows(2).all(|window| {
            self.squares[window[0].0][window[0].1].bind().choice
                == self.squares[window[1].0][window[1].1].bind().choice
        }) {
            for (row, column) in squares {
                self.squares[row][column].bind_mut().is_part_of_solution = true;
            }

            return true;
        }

        false
    }
}

#[derive(GodotClass)]
#[class(base=Area2D)]
struct TicTacToeSquare {
    is_hovered_over: bool,
    is_part_of_solution: bool,
    choice: SquareChoice,

    sprite: Gd<Sprite2D>,
    shape: Gd<CollisionShape2D>,

    base: Base<Area2D>,
}
#[godot_api]
impl IArea2D for TicTacToeSquare {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            is_hovered_over: false,
            is_part_of_solution: false,
            choice: SquareChoice::None,

            sprite: Sprite2D::new_alloc(),
            shape: CollisionShape2D::new_alloc(),

            base,
        }
    }

    fn ready(&mut self) {
        let mut gd = self.to_gd();

        self.sprite.set_hframes(2);
        self.sprite.set_vframes(3);
        self.sprite.set_texture(load("res://Assets/squares.png"));
        gd.add_child(self.sprite.clone().upcast());

        self.shape.set_shape({
            let mut shape = RectangleShape2D::new_gd();

            shape.set_size(Vector2 { x: 16.0, y: 16.0 });

            shape.upcast()
        });
        gd.add_child(self.shape.clone().upcast());

        let on_mouse_entered = gd.callable("on_mouse_entered");
        let on_mouse_exited = gd.callable("on_mouse_exited");
        let on_input_event = gd.callable("on_input_event");
        gd.connect("mouse_entered".into(), on_mouse_entered);
        gd.connect("mouse_exited".into(), on_mouse_exited);
        gd.connect("input_event".into(), on_input_event);
    }

    fn process(&mut self, _delta: f64) {
        self.process_sprite();
    }
}
#[godot_api]
impl TicTacToeSquare {
    #[func]
    fn on_mouse_entered(&mut self) {
        self.is_hovered_over = true;
    }

    #[func]
    fn on_mouse_exited(&mut self) {
        self.is_hovered_over = false;
    }

    #[func]
    fn on_input_event(&mut self, _viewport: Gd<Node>, event: Gd<InputEvent>, _shape_idx: i32) {
        let mut grid: Gd<TicTacToeGrid> = self.base().get_parent().unwrap().cast();
        if grid.bind().game_over_time.is_some() {
            return;
        }

        DIRTY_GRID.store(
            if event.is_action("ui_mouse_click_left".into()) {
                let turn = grid.bind().turn;
                match (self.choice, turn) {
                    (SquareChoice::None, SquareChoice::None | SquareChoice::X) => {
                        self.choice = SquareChoice::X;
                        grid.bind_mut().turn = SquareChoice::O;
                        true
                    }
                    (SquareChoice::X | SquareChoice::O, _)
                    | (SquareChoice::None, SquareChoice::O) => false,
                }
            } else if event.is_action("ui_mouse_click_right".into()) {
                let turn = grid.bind().turn;
                match (self.choice, turn) {
                    (SquareChoice::None, SquareChoice::None | SquareChoice::O) => {
                        self.choice = SquareChoice::O;
                        grid.bind_mut().turn = SquareChoice::X;
                        true
                    }
                    (SquareChoice::X | SquareChoice::O, _)
                    | (SquareChoice::None, SquareChoice::X) => false,
                }
            } else {
                false
            },
            atomic::Ordering::Relaxed,
        )
    }

    fn process_sprite(&self) {
        let mut sprite: Gd<Sprite2D> = self.sprite.to_godot();

        sprite.set_frame_coords(Vector2i {
            x: match self.is_hovered_over || self.is_part_of_solution {
                true => 1,
                false => 0,
            },
            y: match self.choice {
                SquareChoice::None => 0,
                SquareChoice::X => 1,
                SquareChoice::O => 2,
            },
        });
    }
}
