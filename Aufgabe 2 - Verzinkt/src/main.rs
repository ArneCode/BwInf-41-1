use image::{GrayImage, Luma};
use opensimplex_noise_rs::{self, OpenSimplexNoise};
use rand::prelude::*;
use std::{env, f64::consts::PI, ops::Range, rc::Rc, time::Instant};

//Die Größen des erzeugten Bildes
const WIDTH: usize = 1000;
const HEIGHT: usize = 1000;
//Maximale Latenz der Kristalle, gleichzeiting auch die Größe des Ringpuffers
const LATENCY_MAX: usize = 10;
//Die Anzahl der neuen Kristalle
const N_SEEDS: i32 = 40;
//Die Geschwindigkeiten die neue Kristalle haben können (zufällig ausgewählt)
const MAG_RANGE: Range<f64> = (LATENCY_MAX as f64 / 1.1)..LATENCY_MAX as f64;
//Die Helligkeit:
const BRIGHT_MIN: u8 = 50;
const BRIGHT_MAX: u8 = 250;
//Die Geschwindigkeiten die neue Kristalle mindestens haben
const MIN_SPEED_RANGE: Range<f64> = 1.0..2.0;
//Wie "eingezoomt" die Noise-Funktion ist
const NOISE_SCALE: f64 = 0.0002;
//Wie stark die Werte der Noise-Funktion die Simulation beeinflussen
const NOISE_IMPORTANCE: f64 = 5.0;
//Die Wahrscheinlichkeit, dass ein Kristall mutiert
const MUT_PROB: f64 = 0.0001;
//Um wieviel die Helligkeit mutieren kann
const BRIGHT_MUT_RANGE: Range<i32> = -30..30;
//Um wieviel die Wachstumsdauern mutieren können
const LATENCY_MUT_RANGE: Range<i32> = -10..2;

fn save_pixels(pixels: Vec<Vec<u8>>, path: &str) {
    let mut img = GrayImage::new(pixels[0].len() as u32, pixels.len() as u32);
    for (y, row) in pixels.into_iter().enumerate() {
        for (x, value) in row.into_iter().enumerate() {
            img.put_pixel(x as u32, y as u32, Luma([value]));
        }
    }
    img.save(path).unwrap();
}

//Die verschiedenen Richtungen
const DIRS: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
#[derive(Clone, Debug)]
struct Crystal {
    pub brightness: u8,
    //Wartezeiten in der folgenden Reihenfolge: HOCH, RECHTS, RUNTER, LINKS
    pub update_delays: [u8; 4],
}

impl Crystal {
    //Der Konstruktor
    fn new(max_speed: u8, rng: &mut ThreadRng) -> Self {
        let brightness = rng.gen_range(BRIGHT_MIN..BRIGHT_MAX);
        let mag = /*max_speed as f64;*/rng.gen_range(MAG_RANGE);
        let dir = rng.gen_range(0.0..PI * 2.0);
        let update_rates: [f64; 4] = [
            dir.sin() * mag,  //HOCH
            dir.cos() * mag,  //RECHTS
            -dir.sin() * mag, //RUNTER
            -dir.cos() * mag, //LINKS
        ];
        let update_delays: [u8; 4] = update_rates
            .iter()
            //Stellt sicher, dass die Geschwindigkeiten mindestens im Wertebereich MIN_SPEED_RANGE sind
            .map(|rate| (max_speed as f64 / rate.max(rng.gen_range(MIN_SPEED_RANGE))) as u8)
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap();
        Self {
            brightness: brightness as u8,
            update_delays,
        }
    }
    fn mutate(&self, rng: &mut ThreadRng) -> Rc<Self> {
        let brightness = self.brightness as i32 + rng.gen_range(BRIGHT_MUT_RANGE);
        //Stellt sicher, dass die Helligkeit mindestens BRIGHT_MIN und höchstens BRIGHT_MAX ist
        let brightness = brightness.max(BRIGHT_MIN as i32).min(BRIGHT_MAX as i32) as u8;
        let update_delays = self
            .update_delays
            .iter()
            .map(|d| {
                let new = *d as i32 + rng.gen_range(LATENCY_MUT_RANGE);
                new.max(1).min(LATENCY_MAX as i32) as u8 //Die Wartezeit ist mindestens eins und höchstens LATENCY_MAX
            })
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap();
        Rc::new(Self {
            brightness,
            update_delays,
        })
    }
    fn add_to_poss_cells(
        self: &Rc<Crystal>,
        poss_cells: &mut Vec<Vec<(usize, usize, Rc<Crystal>)>>, //Der Ringpuffer
        x: usize,
        y: usize,
        t: usize, //Die momentane Position des Ringpuffers
        rng: &mut ThreadRng,
    ) {
        let mut mutated = false;
        let crystal = if rng.gen_range(0.0..1.0) < MUT_PROB {
            //den Kristall mutieren
            mutated = true;
            self.mutate(rng)
        } else {
            self.clone()
        };
        for (i, dir) in DIRS.iter().enumerate() {
            let delay = if mutated { 1 } else { self.update_delays[i] }; //Mutierte Kristalle schneller erzeugen, sodass weniger "Linien" entstehen
            let new_x = x as i32 + dir.0;
            let new_y = y as i32 + dir.1;
            let len = poss_cells.len();
            //die neue Position in den Ringpuffer delay Stellen weiter einfügen
            poss_cells[((t + delay as usize) % len) as usize].push((
                new_x as usize,
                new_y as usize,
                crystal.clone(),
            ));
        }
    }
}
//Ein Feld des Rasters
type CellState = Option<Rc<Crystal>>;
//Das Raster
struct Grid {
    pub rows: Vec<Vec<CellState>>,
    pub width: usize,
    pub height: usize,
}
impl Grid {
    //Der Konstruktor
    fn new(width: usize, height: usize) -> Self {
        let rows = vec![vec![None; width]; height];
        Self {
            rows,
            width,
            height,
        }
    }
    //Enthält das Raster die Position?
    pub fn contains(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.width || y >= self.height {
            return false;
        }
        true
    }
    pub fn get_unchecked(&self, x: usize, y: usize) -> &CellState {
        &self.rows[y][x]
    }
    pub fn set(&mut self, x: usize, y: usize, crystal: Rc<Crystal>) {
        self.rows[y][x] = CellState::Some(crystal);
    }
    pub fn into_pixels(self) -> Vec<Vec<u8>> {
        self.rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| match cell {
                        Some(cell) => cell.brightness,
                        None => {
                            panic!("Fehler, leeres Kästchen im Raster gefunden")
                        }
                    })
                    .collect()
            })
            .collect()
    }
}
//Generiert die Kristall Keime
fn gen_kristal_seeds(
    grid: &mut Grid,
    rng: &mut ThreadRng,
) -> Vec<Vec<(usize, usize, Rc<Crystal>)>> {
    let mut poss_cells = vec![Vec::new(); LATENCY_MAX as usize];
    let mut n_gen_start = 0;
    let noise_fn = OpenSimplexNoise::new(Some(rng.gen_range(0..10000))); //Die Noise-Funktion initialisieren
    while n_gen_start < N_SEEDS {
        let x = rng.gen_range(0..WIDTH);
        let y = rng.gen_range(0..HEIGHT);
        match grid.get_unchecked(x, y) {
            Some(_) => continue, //nach neuer Position suchen, falls die Position schon besetzt ist
            None => {
                let p = noise_fn.eval_2d((x as f64) * NOISE_SCALE, (y as f64) * NOISE_SCALE); //Wert zwischen -1 und 1
                let p = (p + 1f64) / 2f64; //Wert zwischen 0 und 1
                if rng.gen_range(0.0..1.0) > p.powf(NOISE_IMPORTANCE) {
                    continue;
                }
                n_gen_start += 1;
                let crystal = Rc::new(Crystal::new(LATENCY_MAX as u8, rng));
                crystal.add_to_poss_cells(&mut poss_cells, x, y, 0, rng); //die neuen möglichen Positionen der Kristalle berechnen
                grid.set(x, y, crystal);
            }
        }
    }
    poss_cells
}
//Wachstum der Kristalle simulieren
fn run_simulation(
    poss_cells: &mut Vec<Vec<(usize, usize, Rc<Crystal>)>>,
    mut grid: Grid,
    mut rng: ThreadRng,
) -> Grid {
    let mut is_empty = false; //gibt an, ob es noch mögliche neue Positionen gibt

    //Platzhalter für eine Liste im Ringpuffer
    //Wird aufgrund von rusts "Borrow Checker" benötigt
    let mut cells = Vec::new();
    while !is_empty {
        is_empty = true;
        //Geht jede Liste im Ringpuffer durch
        for t in 0..LATENCY_MAX {
            //tauscht die aktuelle Liste mit dem Platzhalter aus
            std::mem::swap(&mut poss_cells[t], &mut cells);
            if cells.is_empty() {
                continue;
            }
            is_empty = false;
            while let Some((x, y, crystal)) = cells.pop() {
                if !grid.contains(x as i32, y as i32) {
                    continue;
                }
                if grid.get_unchecked(x, y).is_some() {
                    continue;
                }
                //Berechnet neue mögliche Positionen und platziert sie in dem Ringpuffer
                crystal.add_to_poss_cells(poss_cells, x, y, t, &mut rng);
                grid.set(x, y, crystal);
            }
            //Platziert die Liste zurück in den Puffer, sodass der beriets reservierte Platz wiederverwendet werden kann
            std::mem::swap(&mut poss_cells[t], &mut cells);
        }
    }
    grid
}
fn main() {
    //Die Programmargumente
    let args: Vec<String> = env::args().collect();
    //Das Verzeichnis der Ausgabedatei
    let path = if let Some(path) = args.get(1) {
        path
    } else {
        println!("Benutzung:");
        println!("\tcargo run --release <Ausgabedatei>");
        return;
    };
    let now = Instant::now();
    let mut rng = rand::thread_rng();
    let mut grid = Grid::new(WIDTH, HEIGHT);
    let mut poss_cells = gen_kristal_seeds(&mut grid, &mut rng);
    println!("simuliere Wachstum...");
    let grid = run_simulation(&mut poss_cells, grid, rng);
    println!("ferting berechnet, speichere bild in {}", path);
    let pixels = grid.into_pixels();
    save_pixels(pixels, path);
    let elapsed = now.elapsed();
    println!("finished after {:?}", elapsed);
}
