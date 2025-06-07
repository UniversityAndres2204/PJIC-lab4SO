use std::process::exit;
use serde::Deserialize;

const QUANTUM: u32 = 100;
const SWITCH_TIME: u32 = 10;

#[derive(Clone, Default, Debug)]
struct Process {
    id: usize,
    arrival: u32,
    cpu_durations: Vec<u32>,
    cpu_index: usize,
    io_durations: Vec<u32>, // io duration
    io_index: usize,    // current io fase
    next_ready_time: u32,
    remaining_burst_time: u32,
    first_time_cpu: Option<u32>,
    finish_time: Option<u32>,
    waiting_time: u32,
    turn_around_time: u32,
}

impl Process {
    fn new(id: usize, arrival: u32, cpu: Vec<u32>, io: Vec<u32>) -> Self {
        let remaining = cpu[0];
        Process {
            id,
            arrival,
            cpu_durations: cpu,
            cpu_index: 0,
            io_durations: io,
            io_index: 0,
            next_ready_time: arrival,
            remaining_burst_time: remaining,
            first_time_cpu: None,
            finish_time: None,
            waiting_time: 0,
            turn_around_time: 0,
        }
    }
}

#[derive(Clone, Default, Debug, Deserialize)]
struct ProcessEntry {
    arrival: u32,
    cpu_durations: Vec<u32>,
    io_durations: Vec<u32>
}

#[derive(Debug)]
struct RoundRobinResult {
    processes: Vec<Process>,
    total_time: usize,
    average_waiting_time: f32,
    average_turn_around_time: f32,
    queue_history: Vec<usize>,
    gantt_chart: Vec<GanttEvent>, // Eventos para el diagrama de Gant
}

#[derive(Debug, Clone)]
struct GanttEvent {
    start_time: u32,
    end_time: u32,
    process_id: Option<usize>, // None para tiempos de intercambio
    is_switch: bool,
}

fn main() {
    let mut processes = extract_json_processes();
    print_process_table(&mut processes);
    let res = round_robin(&mut processes);
    println!("\nQueue: {:?}", res.queue_history);
    println!(
        "{:>10} |{:>10} |{:>10} |{:>12} |{:>10} |{:>10}",
        "Proceso", "Llegada", "Finish", "Primera CPU", "T. Espera", "T. Vuelta"
    );
    for p in &res.processes {
        println!(
            "{:>10} |{:>10} |{:>10} |{:>12} |{:>10} |{:>10}",
            p.id,
            p.arrival,
            p.finish_time.unwrap_or(0),
            p.first_time_cpu.unwrap_or(0),
            p.waiting_time,
            p.turn_around_time
        );
    }

    println!("\nPromedio espera: {:.2}", res.average_waiting_time);
    println!("Promedio vuelta: {:.2}", res.average_turn_around_time);
    println!("Tiempo total de CPU: {}", res.total_time);
    print_gantt_chart(&res.gantt_chart);
}

fn round_robin(processes: &mut Vec<Process>) -> RoundRobinResult {
    let quantum = QUANTUM;
    let switch_time = SWITCH_TIME;
    let mut current_time: u32 = 0;
    let mut queue_history: Vec<usize> = vec![];
    let mut gantt_chart: Vec<GanttEvent> = Vec::new(); // Eventos del diagrama de Gantt

    while processes.iter().any(|p| p.cpu_index < p.cpu_durations.len()) {
        let mut ready: Vec<_> = processes
            .iter_mut()
            .filter(|p| p.next_ready_time <= current_time && p.cpu_index < p.cpu_durations.len())
            .collect();

        ready.sort_by_key(|p| p.next_ready_time);
        let p = match ready.first_mut() {
            Some(proc) => proc,
            None => {
                current_time += quantum;
                continue;
            }
        };
        queue_history.push(p.id);

        if p.first_time_cpu.is_none() {
            p.first_time_cpu = Some(current_time);
        }

        let burst_time = p.remaining_burst_time.min(quantum);
        p.remaining_burst_time -= burst_time;

        gantt_chart.push(GanttEvent {
            start_time: current_time,
            end_time: current_time + burst_time,
            process_id: Some(p.id),
            is_switch: false,
        });

        current_time += burst_time;
        p.next_ready_time = current_time;

        if p.remaining_burst_time == 0 {
            p.cpu_index += 1;
            if p.cpu_index < p.cpu_durations.len() {
                let io_delay = p.io_durations[p.io_index];
                p.io_index += 1;
                p.remaining_burst_time = p.cpu_durations[p.cpu_index];
                p.next_ready_time = current_time + io_delay;
            } else {
                p.finish_time = Some(current_time);
            }
        }

        if processes.iter().any(|p| p.cpu_index < p.cpu_durations.len()) {
            gantt_chart.push(GanttEvent {
                start_time: current_time,
                end_time: current_time + switch_time,
                process_id: None,
                is_switch: true,
            });
        }
        current_time += switch_time;
    }

    for proc in processes.iter_mut() {
        let finish = proc.finish_time.unwrap_or(current_time);
        proc.turn_around_time = finish - proc.io_durations.iter().sum::<u32>() - proc.arrival;
        proc.waiting_time = proc.first_time_cpu.unwrap() - proc.arrival;
    }

    let total_time = current_time;
    let avg_wait =
        processes.iter().map(|p| p.waiting_time).sum::<u32>() as f32 / processes.len() as f32;
    let avg_turn =
        processes.iter().map(|p| p.turn_around_time).sum::<u32>() as f32 / processes.len() as f32;

    RoundRobinResult {
        processes: processes.to_vec(),
        total_time: total_time as usize,
        average_waiting_time: avg_wait,
        average_turn_around_time: avg_turn,
        queue_history,
        gantt_chart,
    }
}

fn print_gantt_chart(gantt_chart: &[GanttEvent]) {
    println!("\nDiagrama de Gantt:");

    const MAX_WIDTH: usize = 80; // Ancho máximo de la terminal
    let mut current_pos = 0;
    let mut line_events: Vec<&GanttEvent> = Vec::new();

    for event in gantt_chart {
        let event_width = if event.is_switch { 5 } else { 6 }; // "| SW |" vs "| P0 |"

        // Si no cabe en la línea actual, imprimir la línea y empezar una nueva
        if current_pos + event_width > MAX_WIDTH && !line_events.is_empty() {
            print_gantt_line(&line_events);
            line_events.clear();
            current_pos = 0;
        }

        line_events.push(event);
        current_pos += event_width;
    }

    // Imprimir la última línea si queda algo
    if !line_events.is_empty() {
        print_gantt_line(&line_events);
    }
}

fn print_gantt_line(events: &[&GanttEvent]) {
    if events.is_empty() {
        return;
    }

    // Imprimir línea de tiempos
    print!("Tiempo:");
    for event in events {
        print!("{:>4}", event.start_time);
        if event.is_switch {
            print!(" ");
        } else {
            print!("   ");
        }
    }
    // Imprimir el tiempo final del último evento
    if let Some(last_event) = events.last() {
        print!("{:>5}", last_event.end_time);
    }
    println!();

    // Imprimir línea de separación superior
    print!("        ");
    for (i, event) in events.iter().enumerate() {
        if event.is_switch {
            print!("----");
        } else {
            print!("+------+");
        }
        if i == events.len() - 1 && event.is_switch { print!("+"); }
    }
    println!();

    // Imprimir línea de procesos
    print!("        ");
    for event in events {
        if event.is_switch {
            print!("| SW ");
        } else {
            print!("|  P{}  ", event.process_id.unwrap());
        }
    }
    print!("|\n");

    // Imprimir línea de separación inferior
    print!("        ");
    for (i, event) in events.iter().enumerate() {
        if event.is_switch {
            print!("----");
        } else {
            print!("+------+");
        }
        if i == events.len() - 1 && event.is_switch { print!("+"); }
    }
    print!("\n\n");
}

fn print_process_table(processes: &[Process]) {
    // Calcular cuántas ráfagas máximas hay por proceso
    let max_steps = processes
        .iter()
        .map(|p| p.cpu_durations.len() + p.io_durations.len())
        .max()
        .unwrap_or(0);

    // Encabezado
    print!("{:<8} {:<15}", "Proceso", "Llegada (ms)");
    for i in 0..max_steps {
        if i % 2 == 0 {
            print!(" {:<15}", "CPU (quantums)");
        } else {
            print!(" {:<15}", "I/O (quantums)");
        }
    }
    println!();

    // Filas por proceso
    for p in processes {
        print!("{:<8} {:<15}", format!("P{}", p.id), p.arrival);

        let mut cpu_i = 0;
        let mut io_i = 0;

        for i in 0..max_steps {
            if i % 2 == 0 {
                // CPU
                if let Some(cpu) = p.cpu_durations.get(cpu_i) {
                    print!(" {:<15}", cpu/QUANTUM);
                    cpu_i += 1;
                } else {
                    print!(" {:<15}", "-");
                }
            } else {
                // I/O
                if let Some(io) = p.io_durations.get(io_i) {
                    print!(" {:<15}", io/QUANTUM);
                    io_i += 1;
                } else {
                    print!(" {:<15}", "-");
                }
            }
        }

        println!();
    }
}

fn extract_json_processes() -> Vec<Process> {
    let data: Vec<ProcessEntry> = serde_json::from_str(include_str!("processes.json")).unwrap_or_else(|e| {
        println!("{}", e);
        exit(101)
    });
    let mut processes: Vec<Process> = Vec::new();
    for (id, mut entry) in data.into_iter().enumerate() {
        let cpu: Vec<u32> = entry.cpu_durations.iter().map(|d| d*QUANTUM).collect();
        let io: Vec<u32> = entry.io_durations.iter().map(|d| d*QUANTUM).collect();
        processes.push(Process::new(
            id,
            entry.arrival,
            cpu,
            io
        ));
    }
    processes
}