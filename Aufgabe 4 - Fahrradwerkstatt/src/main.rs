use std::{
    cmp::{Ordering, Reverse}, //Wird benutzt, um die Reihenfolge der Aufträge im Heap zu bestimmen
    collections::BinaryHeap,  //der Binäre Heap
    env,
    fs,
};

//Klasse, repräsentiert einen Auftrag
struct Task {
    size: i32,
    time_worked_on: i32,
    start_t: i32,
    latency: i32,
}

impl Task {
    //der Konstruktor der Task Klasse
    fn new(size: i32, time_worked_on: i32, start_t: i32, latency: i32) -> Self {
        Self {
            size,
            time_worked_on,
            start_t,
            latency,
        }
    }
}
//Bestimmt die Reihenfolge der Aufträge im Heap
impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        //Es handlet sich bei dem Binären Heap normalerweise um einen Max-Heap, das heißt größere Elemente kommen zuerst
        //Reverse kehrt diese Folge um, sodass Elemente mit kleinerer Latenz zuerst kommen
        Reverse(self.latency).partial_cmp(&Reverse(other.latency))
    }
}

//von der Implementation des Binären Heaps vorausgesetzt, aber nie tatsächlich genutzt
impl PartialEq for Task {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}
//von der Implementation des Binären Heaps vorausgesetzt, aber nie tatsächlich genutzt
impl Eq for Task {}
//von der Implementation des Binären Heaps vorausgesetzt, aber nie tatsächlich genutzt
impl Ord for Task {
    fn cmp(&self, _other: &Self) -> Ordering {
        todo!()
    }
}

const MINUTES_PER_DAY: i32 = 60 * 24;
const NEUN_UHR: i32 = 9 * 60;
const NEUNZEHN_UHR: i32 = 19 * 60;

fn handle_tasks_in_order(tasks: Vec<[i32; 2]>) {
    println!("Estes Verfahren (Aufträge in Reihenfolge): ");
    handle_tasks(tasks, |start_t, _| start_t)
}
fn handle_tasks_min_first(tasks: Vec<[i32; 2]>) {
    println!("Zweites Verfahren (kleinste Aufträge zuerst): ");
    handle_tasks(tasks, |_, size| size)
}
const F: f64 = 0.1; //Kontrolliert wie sehr das Verfahren den anderen beiden ähnelt.
fn handle_tasks_balanced(tasks: Vec<[i32; 2]>) {
    println!("Eingenes Verfahren: ");
    handle_tasks(tasks, |start_t, size| size + (start_t as f64 * F) as i32)
}
//wandelt eine Minutenanzahl in einen Verständlichen Text um
fn t_to_str(t: i32) -> String {
    format!(
        "{}d {}h {}min",
        t / MINUTES_PER_DAY,
        (t % MINUTES_PER_DAY) / 60,
        (t % MINUTES_PER_DAY) % 60,
    )
}
//Bearbeitet Aufträge in einer bestimmten Reihenfolge
fn handle_tasks(tasks: Vec<[i32; 2]>, latency_fn: fn(i32, i32) -> i32) {
    let mut heap: BinaryHeap<Task> = BinaryHeap::new(); //Speichert eingegangene Aufträge
    let mut durations = Vec::with_capacity(tasks.len()); //Die Wartezeiten

    //Die noch nicht eingegangenen Aufträge. Durch ".peekable()" kann man schauen, wann der nächste Auftrag eingeht, ohne den Iterator zu konsumieren
    let mut tasks = tasks.into_iter().peekable();
    let mut curr_t = 0;
    let mut curr_task: Option<Task> = None;
    loop {
        //immer wieder schauen, wann der nächste Auftrag ankommt
        while let Some([arrival_t, _]) = tasks.peek() {
            //falls der nächste Auftrag noch nicht angekommen ist, wird die Schleife unterbrochen
            if arrival_t > &curr_t {
                break;
            }
            // Den nächsten Auftrag "aufbrauchen" und die Daten erhalten
            let [arrival_t, duration] = tasks.next().unwrap();
            // Den Auftrag zu angekommenen Aufträgen zum Heap hinzufügen
            heap.push(Task::new(
                duration,
                0,
                arrival_t,
                latency_fn(arrival_t, duration), //die Latenz mithilfe der Funktion berechnen
            ));
        }
        if let Some(task) = &mut curr_task {
            //task ist momentan in bearbeitung
            let day_time = curr_t % MINUTES_PER_DAY; //Die Tageszeit
            if (NEUN_UHR..NEUNZEHN_UHR).contains(&day_time) {
                //Der aktuelle Auftrag wird weiterbearbeitet
                task.time_worked_on += 1;
                if task.time_worked_on >= task.size {
                    //Der Auftrag ist fertig bearbeitet
                    let waiting_t = curr_t - task.start_t;
                    durations.push(waiting_t);
                    curr_task = None;
                }
            }
        } else if let Some(task) = heap.pop() {
            //ein neuer Task wird aus dem Heap geholt und jetzt bearbeitet
            curr_task = Some(task);
        } else if tasks.peek().is_none() {
            //keine Aufträge mehr über => fertig
            break;
        }
        //Zeit um eine Minute erhöhem
        curr_t += 1;
    }
    //Die Kennzahlen berechnen
    let total_duration: i32 = durations.iter().sum();
    durations.sort();
    let median = durations[durations.len() / 2];
    let avg_duration = total_duration as f64 / durations.len() as f64;
    let max_duration = durations.iter().max().unwrap();
    //Die Kennzahlen ausgeben
    println!(
        "durchschnittliche Auftragsdauer: {}, max: {}, median: {}",
        t_to_str(avg_duration as i32),
        t_to_str(max_duration.clone()),
        t_to_str(median)
    );
}

fn main() {
    let args: Vec<String> = env::args().collect(); //die Argumente des Programms
    let path = args.get(1).expect("Bitte Dateipfad als Argument angeben"); //Der dateipfad wird aus den Argumenten erhalten
    let data = fs::read_to_string(path).expect("couldn't load data"); //Das Beispiel wird aus der Datei geladen

    //Das Beispiel wird geparsed:
    let tasks: Vec<[i32; 2]> = data
        .lines()
        .filter_map(|line| {
            if line == "" {
                None //komischerweise haben die Beispieleingaben eine leere Zeile am ende
            } else {
                Some(
                    line.split(' ') //"12341 1300" => "12341", "1300"
                        .map(|n| n.parse().unwrap()) //Text wird in Zahlen umgewandelt
                        .collect::<Vec<i32>>()
                        .try_into()
                        .unwrap(),
                )
            }
        })
        .collect();
    handle_tasks_in_order(tasks.clone());
    handle_tasks_min_first(tasks.clone());
    handle_tasks_balanced(tasks);
}
