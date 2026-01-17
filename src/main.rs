#![allow(dead_code)]
use macroquad::prelude::*;
use std::collections::VecDeque;

const MAP: usize = 20;
//  dimensione del tile
const T_SIZE: (f32, f32) = (32., 16.);

// Enum per gestire stati gioco
enum AppState {
    Menu,
    Playing,
    GameOver,
}

// Enum per tile
#[derive(Copy, Clone, PartialEq)]
enum Tile {
    Wall,
    Floor,
}

//Math Helper translate grid to isometric view
fn to_screen(x: usize, y: usize, cam: (f32, f32)) -> (f32, f32) {
    (
        (x as f32 - y as f32) * T_SIZE.0 + cam.0,
        (x as f32 + y as f32) * T_SIZE.1 + cam.1,
    )
}

// Inverso di to_screen: converte coordinate schermo → coordinate griglia
fn to_tile(sx: f32, sy: f32, cam: (f32, f32)) -> (usize, usize) {
    let (ax, ay) = (sx - cam.0, sy - cam.1);
    (
        ((ax / T_SIZE.0 + ay / T_SIZE.1) / 2.) as usize,
        ((ay / T_SIZE.1 - ax / T_SIZE.0) / 2.) as usize,
    )
}

// Pathfinding: Breadth-First Search (BFS)
// Trova il percorso più breve tra start e goal evitando i muri.
// Ritorna un Vec con le coordinate del percorso (escluso start, incluso goal).
// Ritorna vec vuoto se non esiste un percorso.
//
// BFS esplora "a onde concentriche": prima tutte le celle a distanza 1,
// poi quelle a distanza 2, ecc. La coda FIFO (First In, First Out) garantisce
// questo ordine. Se usassimo uno stack LIFO avremmo DFS (Depth-First Search),
// che va "in profondità" e non garantisce il percorso più breve.
fn bfs(
    map: &[[Tile; MAP]; MAP],
    start: (usize, usize),
    goal: (usize, usize),
) -> Vec<(usize, usize)> {
    // Coda FIFO: celle da esplorare. BFS usa FIFO per garantire il percorso più breve.
    let mut q = VecDeque::from([start]);

    // Matrice visited: traccia le celle già visitate per evitare loop infiniti.
    // Senza questo, l'algoritmo continuerebbe a visitare le stesse celle (dead loop).
    let mut visited = [[false; MAP]; MAP];
    visited[start.1][start.0] = true;

    // Matrice parent: per ogni cella, memorizza da quale cella ci siamo arrivati.
    // Serve per ricostruire il percorso una volta raggiunto il goal.
    let mut parent: [[Option<(usize, usize)>; MAP]; MAP] = [[None; MAP]; MAP];

    // Estrai celle dalla coda finché non è vuota
    while let Some(curr) = q.pop_front() {
        // Se abbiamo raggiunto il goal, ricostruiamo il percorso
        if curr == goal {
            let mut path = vec![];
            let mut c = goal;
            // Risaliamo i parent dal goal fino allo start
            while c != start {
                path.push(c);
                c = parent[c.1][c.0].unwrap();
            }
            // Il percorso è al contrario (goal→start), lo invertiamo
            path.reverse();
            return path;
        }

        // Esplora i 4 vicini (su, giù, sinistra, destra)
        for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
            let (nx, ny) = ((curr.0 as i32 + dx) as usize, (curr.1 as i32 + dy) as usize);

            // Controlla: dentro i bounds, non già visitata, non è un muro
            if nx < MAP && ny < MAP && !visited[ny][nx] && map[ny][nx] == Tile::Floor {
                visited[ny][nx] = true; // Marca come visitata PRIMA di aggiungere alla coda
                parent[ny][nx] = Some(curr); // Ricorda da dove siamo arrivati
                q.push_back((nx, ny)); // Aggiungi alla coda per esplorarla dopo
            }
        }
    }
    // Coda vuota e goal non raggiunto = nessun percorso possibile
    vec![]
}

// draw hero and monsters
fn draw_stickman(x: usize, y: usize, cam: (f32, f32)) {
    let (sx, mut sy) = to_screen(x, y, cam);
    sy += 16.;

    // shadow
    draw_ellipse(sx, sy + 3., 10., 5., 0., Color::new(0., 0., 0., 0.2));
    // head
    draw_circle_lines(sx, sy - 32., 7., 2., BLACK);
    // body and limbs
    for l in [
        [0., -25., 0., -8.],
        [0., -20., -8., -15.],
        [0., -20., 8., -15.],
        [0., -8., -6., 0.],
        [0., -8., 6., 0.],
    ] {
        draw_line(sx + l[0], sy + l[1], sx + l[2], sy + l[3], 2., BLACK);
    }
}

// Draw walls
fn draw_wall(x: usize, y: usize, cam: (f32, f32)) {
    let (sx, sy) = to_screen(x, y, cam);

    let v = [
        vec2(sx, sy - 40.),
        vec2(sx + 32., sy - 24.),
        vec2(sx, sy - 8.),
        vec2(sx - 32., sy - 24.),
        vec2(sx + 32., sy),
        vec2(sx, sy + 16.),
        vec2(sx - 32., sy),
    ];

    let colors = [
        Color::new(0.8, 0.8, 0.8, 1.),
        Color::new(0.5, 0.5, 0.5, 1.),
        Color::new(0.6, 0.6, 0.6, 1.),
    ];

    // Draw faces with triangles
    draw_triangle(v[0], v[1], v[2], colors[0]);
    draw_triangle(v[0], v[2], v[3], colors[0]);
    draw_triangle(v[1], v[4], v[5], colors[1]);
    draw_triangle(v[1], v[5], v[2], colors[1]);
    draw_triangle(v[3], v[2], v[5], colors[2]);
    draw_triangle(v[3], v[5], v[6], colors[2]);

    // draw outline
    for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 0), (1, 4), (2, 5), (3, 6)] {
        draw_line(v[a].x, v[a].y, v[b].x, v[b].y, 1., BLACK);
    }
}

struct Game {
    map: [[Tile; MAP]; MAP],
    cam: (f32, f32),
    px: usize,
    py: usize,
    // Percorso calcolato da BFS: lista di celle da attraversare per raggiungere il target
    path: Vec<(usize, usize)>,
}

impl Game {
    fn new() -> Self {
        let mut map = [[Tile::Floor; MAP]; MAP];

        for i in 0..MAP {
            map[0][i] = Tile::Wall;
            map[MAP - 1][i] = Tile::Wall;
            map[i][0] = Tile::Wall;
            map[i][MAP - 1] = Tile::Wall;
        }

        // Add obstacles
        for (x, y) in [(5, 5), (6, 5), (12, 10)] {
            map[y][x] = Tile::Wall;
        }

        Game {
            map,
            cam: (screen_width() / 2., 50.),
            px: 2,
            py: 2,
            path: vec![],
        }
    }

    fn update(&mut self, _dt: f32) -> bool {
        // fake gaming logic
        if is_key_pressed(KeyCode::Space) {
            return true;
        }

        // Input mouse: al click sinistro, calcola il percorso verso la cella cliccata
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            // Converte coordinate schermo → coordinate griglia
            let (tx, ty) = to_tile(mx, my, self.cam);

            // Verifica: dentro i bounds e non è un muro
            if tx < MAP && ty < MAP && self.map[ty][tx] == Tile::Floor {
                // Calcola il percorso con BFS dalla posizione attuale al target
                self.path = bfs(&self.map, (self.px, self.py), (tx, ty));
            }
        }
        false
    }

    fn draw(&self) {
        // draw_text("Game Running....", 20., 40., 30., BLACK);
        // draw_text("Press Space to die...", 20., 80., 20., DARKGRAY);
        for y in 0..MAP {
            for x in 0..MAP {
                if self.map[y][x] == Tile::Wall {
                    draw_wall(x, y, self.cam);
                } else {
                    let (sx, sy) = to_screen(x, y, self.cam);
                    draw_circle(sx, sy + 16., 2., LIGHTGRAY);
                }
            }
        }

        // Disegna il percorso calcolato da BFS come cerchi dorati
        for (px, py) in &self.path {
            let (sx, sy) = to_screen(*px, *py, self.cam);
            draw_circle(sx, sy + 16., 4., GOLD);
        }

        // Draw Player
        draw_stickman(self.px, self.py, self.cam);
    }
}

#[macroquad::main("Crablo")]
async fn main() {
    let mut game = Game::new();
    let mut state = AppState::Menu;

    loop {
        clear_background(WHITE);

        match state {
            AppState::Menu => {
                draw_text("Menu - Enter to start", 100., 100., 40., BLACK);
                if is_key_pressed(KeyCode::Enter) {
                    game = Game::new();
                    state = AppState::Playing;
                }
            }

            AppState::Playing => {
                if game.update(get_frame_time()) {
                    state = AppState::GameOver;
                }
                game.draw();
            }
            AppState::GameOver => {
                game.draw();
                draw_rectangle(
                    0.,
                    0.,
                    screen_width(),
                    screen_width(),
                    Color::new(1., 1., 1., 0.7),
                );
                draw_text("GAME OVER", 100., 100., 60., RED);
                draw_text("Enter to reset", 100., 150., 20., GRAY);

                if is_key_pressed(KeyCode::Enter) {
                    state = AppState::Menu
                }
            }
        }
        next_frame().await;
    }
}
