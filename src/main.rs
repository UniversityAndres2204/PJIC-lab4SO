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

#[derive(Debug)]
struct GanttEvent {
    pid: usize,
    start: u32,
    end: u32,
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
    gantt_events: Vec<GanttEvent>,
}

fn print_gantt_diagram(gantt: &[GanttEvent], switch_time: u32) {
    println!("\nDiagrama de Gantt:");
    
    // Calcular el ancho total necesario
    let total_width = gantt.iter().map(|e| (e.end - e.start) as usize).sum::<usize>();
    let scale_factor = if total_width > 80 { 80.0 / total_width as f64 } else { 1.0 };
    
    // Línea de tiempo superior
    print!("0");
    for event in gantt {
        let width = ((event.end - event.start) as f64 * scale_factor).max(1.0) as usize;
        let padding = if width >= 8 { width - 4 } else { 4 };
        print!("{:>width$}", event.end, width = padding);
    }
    println!();
    
    // Línea superior del diagrama

    for event in gantt {
        let width = ((event.end - event.start) as f64 * scale_factor).max(1.0) as usize;
        print!("+{:-<width$}", "", width = width.saturating_sub(1));
    }
    println!("+");
    
    // Línea de procesos

    for event in gantt {
        let width = ((event.end - event.start) as f64 * scale_factor).max(1.0) as usize;
        let label = if event.pid == usize::MAX {
            "SW".to_string()
        } else {
            format!("P{}", event.pid)
        };
        
        if width >= label.len() + 2 {
            let padding = (width - label.len()) / 2;
            print!("|{:>pad$}{:<pad2$}", label, "", pad = padding, pad2 = width - padding - label.len());
        } else {
            print!("|{:<width$}", label.chars().take(width.saturating_sub(1)).collect::<String>(), width = width.saturating_sub(1));
        }
    }
    println!("|");
    
    // Línea inferior del diagrama

    for event in gantt {
        let width = ((event.end - event.start) as f64 * scale_factor).max(1.0) as usize;
        print!("+{:-<width$}", "", width = width.saturating_sub(1));
    }
    println!("+");
    
    // Leyenda
    println!("\nLeyenda:");
    println!("  P# = Proceso # ejecutándose");
    println!("  SW = Context Switch (Intercambio entre procesos)");
    println!("\nExplicación:");
    println!("- Cada vez que un proceso agota su quantum o completa su ráfaga,");
    println!("  ocurre un context switch (SW) antes del siguiente proceso.");
    println!("- El context switch toma {} unidades de tiempo.", switch_time);
}

fn rust_round_robin(mut processes: Vec<Process>, quantum: usize, switch_time: u32) -> RoundRobinResult {
    println!("[+] Round Robin on rust!");
    let mut current_time = 0;
    let mut queue = VecDeque::new();
    let mut io_queue: Vec<(usize, u32)> = Vec::new();
    let mut ready_processes: HashSet<usize> = HashSet::new();
    let mut gantt: Vec<GanttEvent> = Vec::new();

    while queue.len() > 0 || ready_processes.len() < processes.len() || !io_queue.is_empty() {
        // Agregar procesos que llegan
        for (i, process) in processes.iter_mut().enumerate() {
            if process.arrival <= current_time && !ready_processes.contains(&i) {
                queue.push_back(i);
                ready_processes.insert(i);
            }
        }

        // Procesar I/O completado
        let mut finished_io = vec![];
        for (i, &(pid, end_io)) in io_queue.iter().enumerate() {
            if current_time >= end_io {
                queue.push_back(pid);
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
        let start_time = current_time;
        process.remaining_burst_time -= burst_time as u32;
        current_time += burst_time as u32;
        
        // Registrar evento de CPU
        gantt.push(GanttEvent {
            pid,
            start: start_time,
            end: current_time,
        });

        // Siempre agregar context switch después de cada ráfaga de CPU (excepto al final)
        let is_last_execution = queue.is_empty() && io_queue.is_empty() && 
                               (process.remaining_burst_time == 0 && process.finished());
        
        if !is_last_execution && switch_time > 0 {
            gantt.push(GanttEvent {
                pid: usize::MAX, // Context switch
                start: current_time,
                end: current_time + switch_time,
            });
            current_time += switch_time;
        }

        if process.remaining_burst_time == 0 {
            // El proceso ha completado su ráfaga actual
            process.io_fase += 1;
            if process.finished() {
                // El proceso ha terminado completamente
                process.finish_time = Some(current_time);
                process.turn_around_time = current_time - process.arrival;
                process.waiting_time = process.turn_around_time - process.ncpu_fases.iter().sum::<u32>() - process.io_time.iter().sum::<u32>();
            } else {
                // El proceso va a I/O
                let io_time = process.io_time[process.io_fase-1];
                process.remaining_burst_time = process.ncpu_fases[process.io_fase];
                io_queue.push((pid, current_time + io_time));
            }
        } else {
            // El proceso fue interrumpido por el quantum, vuelve a la cola
            queue.push_back(pid);
        }
    }

    // Calcular estadísticas finales
    for proc in &mut processes {
        let finish = proc.finish_time.unwrap_or(current_time);
        proc.turn_around_time = finish - proc.arrival;
        let total_cpu_time: u32 = proc.ncpu_fases.iter().sum();
        proc.waiting_time = proc.turn_around_time - total_cpu_time;
    }

    let total_time = current_time;
    let avg_wait = processes.iter().map(|p| p.waiting_time).sum::<u32>() as f32 / processes.len() as f32;
    let avg_turn = processes.iter().map(|p| p.turn_around_time).sum::<u32>() as f32 / processes.len() as f32;

    // Mostrar el diagrama de Gantt mejorado
    print_gantt_diagram(&gantt, switch_time);

    RoundRobinResult {
        processes,
        total_time: total_time as usize,
        average_waiting_time: avg_wait,
        average_turn_around_time: avg_turn,
        gantt_events: gantt,
    }
}