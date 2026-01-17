// Crablo - Clone di Diablo in Rust
// Bootcamp Rust parte 2 - Francesco Ciulla

#![allow(dead_code)]
use macroquad::prelude::*;
use std::collections::VecDeque;

// Dimensione della griglia di gioco (20x20 celle)
const MAP: usize = 20;
// Dimensione del tile isometrico (larghezza, altezza)
// La vista isometrica usa un rapporto 2:1 (32 pixel largo, 16 alto)
const T_SIZE: (f32, f32) = (32., 16.);

// Enum per gestire gli stati del gioco (state machine)
enum AppState {
    Menu,     // Schermata iniziale
    Playing,  // Partita in corso
    GameOver, // Fine partita
}

// Enum per i tipi di celle della mappa
#[derive(Copy, Clone, PartialEq)]
enum Tile {
    Wall,  // Muro: blocca il movimento
    Floor, // Pavimento: calpestabile
}

// Struttura per i mostri nemici
struct Monster {
    x: usize, // Posizione X sulla griglia
    y: usize, // Posizione Y sulla griglia
    hp: i32,  // Punti vita
    cd: f32,  // Cooldown per azioni (attacco/movimento)
}

// Struttura per il testo fluttuante del danno (floating damage text)
// Mostra "-10" che sale e scompare quando colpisci un mostro
struct DmgText {
    x: f32,    // Posizione X sullo schermo
    y: f32,    // Posizione Y sullo schermo (sale nel tempo)
    dmg: i32,  // Quantità di danno da mostrare
    life: f32, // Tempo rimanente prima che il testo scompaia (in secondi)
}

// Converte coordinate griglia (x, y) → coordinate schermo (sx, sy)
// Formula isometrica: la X schermo dipende dalla differenza (x-y),
// la Y schermo dipende dalla somma (x+y). cam è l'offset della camera.
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

// Calcola la distanza Manhattan tra due punti sulla griglia
// La distanza Manhattan è la somma delle differenze assolute delle coordinate:
// |x1-x2| + |y1-y2|. Si chiama così perché rappresenta la distanza percorsa
// in una griglia (come le strade di Manhattan), dove puoi muoverti solo
// in orizzontale o verticale, mai in diagonale.
// Usata per determinare se un mostro è adiacente al player (distanza = 1)
fn dist(p1: (usize, usize), p2: (usize, usize)) -> i32 {
    (p1.0 as i32 - p2.0 as i32).abs() + (p1.1 as i32 - p2.1 as i32).abs()
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

// Disegna uno stickman (player o mostro)
// enemy=true: disegna con corna (mostro), enemy=false: disegna con testa tonda (player)
fn draw_stickman(x: usize, y: usize, cam: (f32, f32), enemy: bool) {
    let (sx, mut sy) = to_screen(x, y, cam);
    sy += 16.;

    // Ombra a terra
    draw_ellipse(sx, sy + 3., 10., 5., 0., Color::new(0., 0., 0., 0.2));

    // Testa: corna per i nemici, cerchio per il player
    if enemy {
        // Corna del mostro (due linee a V)
        draw_line(sx - 5., sy - 32., sx, sy - 30., 2., BLACK);
        draw_line(sx + 5., sy - 32., sx, sy - 30., 2., BLACK);
    } else {
        // Testa tonda del player
        draw_circle_lines(sx, sy - 32., 7., 2., BLACK);
    }
    // Corpo e arti: array di linee [x1, y1, x2, y2] relative a (sx, sy)
    // Linea 0: corpo (collo → bacino)
    // Linee 1-2: braccia (spalla → mano sinistra/destra)
    // Linee 3-4: gambe (bacino → piede sinistro/destro)
    for l in [
        [0., -25., 0., -8.],   // corpo
        [0., -20., -8., -15.], // braccio sinistro
        [0., -20., 8., -15.],  // braccio destro
        [0., -8., -6., 0.],    // gamba sinistra
        [0., -8., 6., 0.],     // gamba destra
    ] {
        draw_line(sx + l[0], sy + l[1], sx + l[2], sy + l[3], 2., BLACK);
    }
}

// Disegna un muro 3D isometrico (cubo con 3 facce visibili)
// Il muro è composto da triangoli per creare l'effetto 3D
fn draw_wall(x: usize, y: usize, cam: (f32, f32)) {
    let (sx, sy) = to_screen(x, y, cam);

    // Vertici del cubo isometrico (7 punti)
    // v[0]: punto più alto (cima del cubo)
    // v[1-3]: bordi superiori (destra, fronte, sinistra)
    // v[4-6]: bordi inferiori (destra, fronte, sinistra)
    let v = [
        vec2(sx, sy - 40.),       // 0: top
        vec2(sx + 32., sy - 24.), // 1: top-right
        vec2(sx, sy - 8.),        // 2: top-front
        vec2(sx - 32., sy - 24.), // 3: top-left
        vec2(sx + 32., sy),       // 4: bottom-right
        vec2(sx, sy + 16.),       // 5: bottom-front
        vec2(sx - 32., sy),       // 6: bottom-left
    ];

    // Colori per le 3 facce visibili (illuminazione simulata)
    let colors = [
        Color::new(0.8, 0.8, 0.8, 1.), // top: più chiaro
        Color::new(0.5, 0.5, 0.5, 1.), // right: più scuro
        Color::new(0.6, 0.6, 0.6, 1.), // left: medio
    ];

    // Disegna le 3 facce con triangoli (2 triangoli per faccia)
    // Faccia superiore (top)
    draw_triangle(v[0], v[1], v[2], colors[0]);
    draw_triangle(v[0], v[2], v[3], colors[0]);
    // Faccia destra
    draw_triangle(v[1], v[4], v[5], colors[1]);
    draw_triangle(v[1], v[5], v[2], colors[1]);
    // Faccia sinistra
    draw_triangle(v[3], v[2], v[5], colors[2]);
    draw_triangle(v[3], v[5], v[6], colors[2]);

    // Disegna i bordi neri per definire il contorno
    for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 0), (1, 4), (2, 5), (3, 6)] {
        draw_line(v[a].x, v[a].y, v[b].x, v[b].y, 1., BLACK);
    }
}

// Struttura principale del gioco: contiene tutto lo stato di una partita
struct Game {
    map: [[Tile; MAP]; MAP], // Griglia della mappa (Wall o Floor)
    cam: (f32, f32),         // Offset camera per centrare la vista
    px: usize,               // Posizione X del player sulla griglia
    py: usize,               // Posizione Y del player sulla griglia
    // Percorso calcolato da BFS: lista di celle da attraversare per raggiungere il target
    path: Vec<(usize, usize)>,
    // Cooldown movimento: tempo rimanente prima del prossimo passo (in secondi)
    player_cd: f32,
    // Lista dei mostri presenti nella mappa
    monsters: Vec<Monster>,
    // Lista dei testi di danno fluttuanti attivi
    texts: Vec<DmgText>,
    // Punti vita del player (game over quando <= 0)
    hp: i32,
}

impl Game {
    // Crea una nuova partita con mappa, player e mostri inizializzati
    fn new() -> Self {
        // Inizializza tutta la mappa come pavimento
        let mut map = [[Tile::Floor; MAP]; MAP];

        // Crea i muri perimetrali (bordi della mappa)
        for i in 0..MAP {
            map[0][i] = Tile::Wall; // bordo superiore
            map[MAP - 1][i] = Tile::Wall; // bordo inferiore
            map[i][0] = Tile::Wall; // bordo sinistro
            map[i][MAP - 1] = Tile::Wall; // bordo destro
        }

        // Aggiungi ostacoli interni (muri sparsi)
        for (x, y) in [(5, 5), (6, 5), (12, 10)] {
            map[y][x] = Tile::Wall;
        }

        Game {
            map,
            cam: (screen_width() / 2., 50.),
            px: 2,
            py: 2,
            path: vec![],
            player_cd: 0.,
            // Spawn dei mostri in posizioni fisse sulla mappa
            monsters: vec![
                Monster {
                    x: 8,
                    y: 8,
                    hp: 30,
                    cd: 0.,
                },
                Monster {
                    x: 12,
                    y: 4,
                    hp: 30,
                    cd: 0.,
                },
                Monster {
                    x: 15,
                    y: 12,
                    hp: 30,
                    cd: 0.,
                },
            ],
            texts: vec![],
            hp: 100, // Player inizia con 100 HP
        }
    }

    // Aggiorna lo stato del gioco ogni frame
    // dt = delta time (tempo trascorso dall'ultimo frame)
    // Ritorna true se il gioco deve terminare (game over)
    fn update(&mut self, dt: f32) -> bool {
        // Controllo game over: se HP <= 0, la partita finisce
        if self.hp <= 0 {
            return true;
        }

        // Aggiorna animazione testi di danno fluttuanti
        // retain_mut mantiene solo i testi con life > 0, rimuovendo quelli scaduti
        self.texts.retain_mut(|t| {
            t.life -= dt; // Decrementa il tempo di vita
            t.y -= 20. * dt; // Fa salire il testo verso l'alto
            t.life > 0. // Ritorna true se il testo deve rimanere
        });

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

        // Movimento del player lungo il percorso BFS
        // Usa un cooldown per controllare la velocità (0.15s tra ogni passo)
        if !self.path.is_empty() {
            // Decrementa il cooldown in base al delta time (tempo tra frame)
            self.player_cd -= dt;

            // Quando il cooldown arriva a 0, è ora di muoversi
            if self.player_cd <= 0. {
                // Reset del cooldown per il prossimo passo
                self.player_cd = 0.15;

                // Prossima cella nel percorso
                let (nx, ny) = self.path[0];

                // Logica di combattimento: controlla se c'è un mostro nella prossima cella
                // iter().position() cerca l'indice del primo mostro che occupa (nx, ny)
                if let Some(i) = self.monsters.iter().position(|m| m.x == nx && m.y == ny) {
                    // Mostro trovato! Attacca invece di muoversi
                    self.damage_monster(i, 10);
                    // Ferma il movimento (il player deve cliccare di nuovo)
                    self.path.clear();
                } else {
                    // Nessun mostro: muovi il player nella cella
                    self.path.remove(0);
                    self.px = nx;
                    self.py = ny;
                }
            }
        }

        // Logica dei Mostri
        //
        // NOTA SULLE CLOSURE IN RUST:
        // Una closure è una funzione anonima che può "catturare" variabili dall'ambiente circostante.
        // Sintassi: |parametri| espressione  oppure  |parametri| { blocco }
        //
        // Esempi:
        //   |x| x * 2           → prende x, ritorna x * 2
        //   |a, b| a + b        → prende due parametri, ritorna la somma
        //   |m| (m.x, m.y)      → prende un Monster, ritorna una tupla con le sue coordinate
        //
        // Perché si usano:
        // - Passare logica custom a funzioni come map(), filter(), retain()
        // - Sono concise: evitano di definire funzioni separate per operazioni semplici
        // - Possono accedere a variabili locali (es. self.px, self.py nel chain sotto)
        //
        // Equivalente JavaScript: (x) => x * 2  oppure  function(x) { return x * 2; }
        //
        // Calcola le celle occupate per evitare che i mostri si sovrappongano
        let occupied: Vec<_> = self
            .monsters
            .iter()
            .map(|m| (m.x, m.y)) // Closure: trasforma ogni Monster in una tupla (x, y)
            .chain(std::iter::once((self.px, self.py))) // Aggiungi la posizione del player
            .collect();

        // AI dei mostri: ogni mostro agisce quando il suo cooldown raggiunge 0
        for i in 0..self.monsters.len() {
            // Decrementa il cooldown del mostro
            self.monsters[i].cd -= dt;

            // Quando il cooldown arriva a 0, il mostro può agire
            if self.monsters[i].cd <= 0. {
                // Reset cooldown: il mostro agirà di nuovo tra 1 secondo
                self.monsters[i].cd = 1.0;

                let (mx, my) = (self.monsters[i].x, self.monsters[i].y);

                // Calcola la distanza Manhattan dal player
                let d = dist((mx, my), (self.px, self.py));

                if d == 1 {
                    // Mostro adiacente al player (distanza 1): ATTACCA!
                    self.hp -= 5;
                    // Mostra il danno subito dal player
                    let (sx, sy) = to_screen(self.px, self.py, self.cam);
                    self.texts.push(DmgText {
                        x: sx,
                        y: sy,
                        dmg: 5,
                        life: 1.,
                    });
                } else {
                    // Mostro lontano: INSEGUI il player usando BFS
                    let path = bfs(&self.map, (mx, my), (self.px, self.py));
                    // Muovi solo se c'è un percorso e la cella non è occupata
                    if path.len() > 1 && !occupied.contains(&path[0]) {
                        self.monsters[i].x = path[0].0;
                        self.monsters[i].y = path[0].1;
                    }
                }
            }
        }

        false
    }

    // Infligge danno a un mostro e gestisce la sua morte
    // idx: indice del mostro nel vettore monsters
    // amount: quantità di danno da infliggere
    fn damage_monster(&mut self, idx: usize, amount: i32) {
        // Sottrai HP al mostro
        self.monsters[idx].hp -= amount;

        // Crea il testo fluttuante del danno sopra il mostro
        let (sx, sy) = to_screen(self.monsters[idx].x, self.monsters[idx].y, self.cam);
        self.texts.push(DmgText {
            x: sx,
            y: sy - 40., // Parte sopra la testa del mostro
            dmg: amount,
            life: 1., // Dura 1 secondo
        });

        // Se HP <= 0, il mostro muore: rimuovilo dal vettore
        if self.monsters[idx].hp <= 0 {
            self.monsters.remove(idx);
        }
    }

    // Disegna tutti gli elementi del gioco sullo schermo
    fn draw(&self) {
        // Disegna la mappa: itera su tutte le celle della griglia
        // L'ordine (y poi x) garantisce il corretto z-ordering isometrico
        for y in 0..MAP {
            for x in 0..MAP {
                if self.map[y][x] == Tile::Wall {
                    // Cella muro: disegna cubo 3D
                    draw_wall(x, y, self.cam);
                } else {
                    // Cella pavimento: disegna un piccolo punto grigio
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

        // Disegna il player (enemy=false → testa tonda)
        draw_stickman(self.px, self.py, self.cam, false);

        // Disegna tutti i mostri (enemy=true → corna)
        for m in &self.monsters {
            draw_stickman(m.x, m.y, self.cam, true);
        }

        // Disegna i testi di danno fluttuanti (es. "-10" in rosso che sale)
        for t in &self.texts {
            draw_text(&format!("-{}", t.dmg), t.x, t.y, 20., RED);
        }

        // HUD (Head-Up Display): mostra le statistiche del player
        // Posizionato in basso a sinistra dello schermo
        draw_text(
            &format!("HP: {}", self.hp),
            20.,
            screen_height() - 40.,
            30.,
            BLACK,
        );
    }
}

// Entry point del gioco - macroquad gestisce il window e il game loop
#[macroquad::main("Crablo")]
async fn main() {
    let mut game = Game::new();
    let mut state = AppState::Menu;

    // Game loop principale: gira finché la finestra è aperta
    loop {
        // Pulisce lo schermo con sfondo bianco
        clear_background(WHITE);

        // State machine: gestisce i diversi stati del gioco
        match state {
            // Schermata menu iniziale
            AppState::Menu => {
                draw_text("Menu - Enter to start", 100., 100., 40., BLACK);
                if is_key_pressed(KeyCode::Enter) {
                    // Crea nuova partita e passa allo stato Playing
                    game = Game::new();
                    state = AppState::Playing;
                }
            }

            // Gioco in corso
            AppState::Playing => {
                // get_frame_time() ritorna il delta time per movimento fluido
                if game.update(get_frame_time()) {
                    // Se update() ritorna true, passa a GameOver
                    state = AppState::GameOver;
                }
                game.draw();
            }

            // Schermata game over
            AppState::GameOver => {
                // Disegna il gioco "congelato" sotto l'overlay
                game.draw();
                // Overlay bianco semi-trasparente
                draw_rectangle(
                    0.,
                    0.,
                    screen_width(),
                    screen_width(),
                    Color::new(1., 1., 1., 0.7),
                );
                draw_text("GAME OVER", 100., 100., 60., RED);
                // Mostra gli HP finali del player (sarà <= 0)
                draw_text(&format!("HP:{}", game.hp), 100., 160., 30., BLACK);
                draw_text("Enter to reset", 100., 200., 20., GRAY);

                if is_key_pressed(KeyCode::Enter) {
                    // Torna al menu per iniziare una nuova partita
                    state = AppState::Menu
                }
            }
        }
        // Aspetta il prossimo frame (necessario per macroquad async)
        next_frame().await;
    }
}
