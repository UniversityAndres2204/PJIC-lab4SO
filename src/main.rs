fn main() {
    let processes = vec!(
            Process::new("process1", 0, 2, vec!()),
            Process::new("process2", 2, 2, vec!()),
            Process::new("process3", 4, 2, vec!()),
        );
    let res = rust_round_robin(2, processes);
    println!("{:?}", res);
}

#[derive(Clone, Default, Debug)]
struct Process {
    name: String,
    arrival: usize,
    burst_time: usize,
    remaining_burst_time: usize,
    io: Vec<IOCPU>,
    waiting_time: usize,
    turn_around_time: usize
}

#[derive(Clone, Debug)]
struct IOCPU {
    io_time: usize,
    next_burst_time: usize
}

impl Process {
    fn new<'a>(name: &'a str, arrival: usize, burst_time: usize, io: Vec<IOCPU>) -> Self {
        Self {
            name: name.to_string(),
            arrival, 
            burst_time,
            remaining_burst_time: burst_time,
            io,
            waiting_time: 0,
            turn_around_time: 0
        }
    }
}

#[derive(Debug)]
struct RoundRobinResult {
    processes: Vec<Process>,
    total_time: usize,
    average_waiting_time: usize,
    average_turn_arround_time: usize
}

fn rust_round_robin(quantum: usize, mut processes: Vec<Process>) -> RoundRobinResult {
    // ready queue [/p1, p2, p3, ...]
    println!("[+] Round Robin on rust!");
    let mut total_time = 0;
    let mut completed = 0;
    let mut average_turn_arround_time = 0;
    let mut average_waiting_time = 0;
    let n: usize = processes.len();

    //let mut queue = vec![];
    let mut counter = 0;

    while completed < n {
        dbg!(completed, n, total_time);
        for process in processes.iter_mut() {
            //if process.remaining_burst_time != 0 { println!("Current Process -> {:?}", process) };
            if process.remaining_burst_time > 0 && process.arrival <= total_time {
                if process.remaining_burst_time > quantum {
                    process.remaining_burst_time -= quantum;
                    total_time += quantum;
                    println!("Running -> {:?}", process);
                } else {
                    total_time += process.remaining_burst_time;
                    process.waiting_time = total_time - process.burst_time;
                    process.turn_around_time = total_time - process.arrival;
                    process.remaining_burst_time = 0;

                    average_waiting_time += process.waiting_time;
                    average_turn_arround_time += process.turn_around_time;

                    completed += 1;
                    println!("END -> {:?}", process);
                }
            }

            if counter == 20 { break };
            counter += 1;
            println!("-------------------------------");
        }
    }

    RoundRobinResult {
        processes,
        total_time,
        average_waiting_time: average_waiting_time / n,
        average_turn_arround_time: average_turn_arround_time / n
    }
}
