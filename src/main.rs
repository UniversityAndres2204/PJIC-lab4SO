use std::collections::{HashSet, VecDeque};

fn main() {
    let processes = vec![
        Process::new(0, 0, vec![200, 100, 300], vec![200, 200]),
        Process::new(1, 80, vec![300, 200], vec![200]),
        Process::new(2, 140, vec![500], vec![]),
    ];

    let quantum = 100;
    let switch_time = 10;
    let res = rust_round_robin(processes, quantum, switch_time);

    println!("Proceso\tLlegada\tFinish\tPrimera CPU\tT. Espera\tT. Vuelta");
    for p in &res.processes {
        println!("{}\t{}\t\t{}\t{}\t\t{}\t\t{}",
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
    
    // Mostrar historial de la cola de procesos
    println!("{:?}", res.execution_history);
    
    // Mostrar diagrama de Gantt
    print_gantt_chart(&res.gantt_chart);
}

#[derive(Clone, Default, Debug)]
struct Process {
    id: usize,
    arrival: u32,
    ncpu_fases: Vec<u32>,
    io_time: Vec<u32>, // io duration
    io_fase: usize, // current io fase
    remaining_burst_time: u32,
    first_time_cpu: Option<u32>,
    finish_time: Option<u32>,
    waiting_time: u32,
    turn_around_time: u32
}

impl Process {
    fn new(id: usize, arrival: u32, cpu: Vec<u32>, io: Vec<u32>) -> Self {
        let remaining = cpu.first().copied().unwrap_or(0);
        Process {
            id,
            arrival,
            ncpu_fases: cpu,
            io_time: io,
            io_fase: 0,
            remaining_burst_time: remaining,
            first_time_cpu: None,
            finish_time: None,
            waiting_time: 0,
            turn_around_time: 0,
        }
    }

    fn finished(&self) -> bool {
        self.io_fase >= self.ncpu_fases.len()
    }
}

#[derive(Debug)]
struct RoundRobinResult {
    processes: Vec<Process>,
    total_time: usize,
    average_waiting_time: f32,
    average_turn_around_time: f32,
    execution_history: Vec<usize>, // Historial de IDs de procesos
    gantt_chart: Vec<GanttEvent>, // Eventos para el diagrama de Gantt
}

#[derive(Debug, Clone)]
struct GanttEvent {
    start_time: u32,
    end_time: u32,
    process_id: Option<usize>, // None para tiempos de intercambio
    is_switch: bool,
}

fn rust_round_robin(mut processes: Vec<Process>, quantum: usize, switch_time: u32) -> RoundRobinResult {
    println!("[+] Round Robin on rust!");
    let mut current_time = 0;
    let mut queue = VecDeque::new();
    let mut io_queue: Vec<(usize, u32)> = Vec::new();
    let mut ready_processes: HashSet<usize> = HashSet::new();
    let mut saved_queue: VecDeque<usize> = VecDeque::new(); // Historial de la cola
    let mut gantt_chart: Vec<GanttEvent> = Vec::new(); // Eventos del diagrama de Gantt

    let mut total_turn_around_time = 0;
    let mut total_waiting_time = 0;

    while queue.len() > 0 || ready_processes.len() < processes.len() || !io_queue.is_empty() {
        // Agregar procesos que llegan en este momento
        for (i, process) in processes.iter_mut().enumerate() {
            if process.arrival <= current_time && !ready_processes.contains(&i) {
                queue.push_back(i);
                saved_queue.push_back(i); // Guardar en el historial
                ready_processes.insert(i);
            }
        }

        // Agregar procesos que terminan I/O
        let mut finished_io = vec![];
        for (i, &(pid, end_io)) in io_queue.iter().enumerate() {
            if current_time >= end_io {
                queue.push_back(pid);
                saved_queue.push_back(pid); // Guardar en el historial
                finished_io.push(i);
            }
        }

        for i in finished_io.iter().rev() {
            io_queue.remove(*i);
        }

        if queue.is_empty() {
            current_time += 1;
            continue;
        }

        let pid = queue.pop_front().unwrap();
        let process = &mut processes[pid];

        if process.first_time_cpu.is_none() {
            process.first_time_cpu = Some(current_time);
        }

        let burst_time = quantum.min(process.remaining_burst_time as usize);
        process.remaining_burst_time -= burst_time as u32;
        
        // Agregar evento de ejecución del proceso
        gantt_chart.push(GanttEvent {
            start_time: current_time,
            end_time: current_time + burst_time as u32,
            process_id: Some(pid),
            is_switch: false,
        });
        
        current_time += burst_time as u32;

        // Agregar evento de intercambio si no es el último proceso
        if switch_time > 0 && (queue.len() > 0 || process.remaining_burst_time > 0) {
            gantt_chart.push(GanttEvent {
                start_time: current_time,
                end_time: current_time + switch_time,
                process_id: None,
                is_switch: true,
            });
        }
        
        current_time += switch_time;

        if process.remaining_burst_time == 0 {
            process.io_fase += 1;
            if process.finished() {
                process.finish_time = Some(current_time);
                process.turn_around_time = current_time - process.arrival;
                process.waiting_time = process.turn_around_time - process.ncpu_fases.iter().sum::<u32>() - process.io_time.iter().sum::<u32>();
            } else {
                let io_time = process.io_time[process.io_fase-1];
                process.remaining_burst_time = process.ncpu_fases[process.io_fase];
                io_queue.push((pid, current_time + io_time));
            }
        } else {
            queue.push_back(pid);
            saved_queue.push_back(pid); // Guardar cuando regresa a la cola
        }
    }

    for proc in &mut processes {
        let finish = proc.finish_time.unwrap_or(current_time);
        proc.turn_around_time = finish - proc.arrival;
        let total_cpu_time: u32 = proc.ncpu_fases.iter().sum();
        proc.waiting_time = proc.turn_around_time - total_cpu_time;
    }

    let total_time = current_time;
    let avg_wait = processes.iter().map(|p| p.waiting_time).sum::<u32>() as f32 / processes.len() as f32;
    let avg_turn = processes.iter().map(|p| p.turn_around_time).sum::<u32>() as f32 / processes.len() as f32;

    // Convertir saved_queue a Vec para el resultado
    let execution_history: Vec<usize> = saved_queue.into_iter().collect();

    RoundRobinResult {
        processes,
        total_time: total_time as usize,
        average_waiting_time: avg_wait,
        average_turn_around_time: avg_turn,
        execution_history, // Incluir el historial como Vec<usize>
        gantt_chart, // Incluir el diagrama de Gantt
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
    for event in events {
        if event.is_switch {
            print!("----");
        } else {
            print!("+------+");
        }
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
    println!();
    
    // Imprimir línea de separación inferior
    print!("        ");
    for event in events {
        if event.is_switch {
            print!("----");
        } else {
            print!("+------+");
        }
    }
    println!();
    println!(); // Línea en blanco entre secciones
}