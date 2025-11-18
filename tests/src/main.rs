// tests/client_test.rs
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

struct TestClient {
    process: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl TestClient {
    fn new() -> Self {
        // Build the test_client binary first
        let build_status = Command::new("cargo")
            .args(["build", "-p", "test_client"])
            .status()
            .expect("Failed to build test_client");

        assert!(build_status.success(), "Failed to build test_client binary");

        // Run the binary directly
        let mut process = Command::new("./target/debug/test_client")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to start test client");

        let stdin = process.stdin.take().expect("Failed to open stdin");
        let stdout = BufReader::new(process.stdout.take().expect("Failed to open stdout"));

        TestClient {
            process,
            stdin,
            stdout,
        }
    }

    fn send_command(&mut self, command: &str) {
        writeln!(self.stdin, "{}", command).expect("Failed to write to stdin");
        println!("[SENT] {}", command);
    }

    fn read_output(&mut self) -> String {
        let mut output = String::new();
        self.stdout
            .read_line(&mut output)
            .expect("Failed to read stdout");
        let output = output.trim().to_string();
        if !output.is_empty() {
            println!("[RECV] {}", output);
        }
        output
    }

    fn wait_for_connection(&mut self) {
        println!("Waiting for connection confirmation...");
        loop {
            let output = self.read_output();
            if output.contains("Connected to") {
                println!("✓ Client connected");
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_for_sync(&mut self) -> String {
        println!("Waiting for SYNC message...");
        loop {
            let output = self.read_output();
            if output.starts_with("SYNC {") {
                println!("✓ SYNC received");
                return output;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_for_initial_sync(&mut self) -> String {
        println!("Waiting for initial SYNC...");
        self.wait_for_sync()
    }

    fn wait_for_op_sent(&mut self) {
        println!("Waiting for OP_SENT...");
        loop {
            let output = self.read_output();
            if output.contains("OP_SENT") {
                println!("✓ OP_SENT received");
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

fn main() {
    println!("=== Starting Client Integration Test ===");

    // Spawn two test clients
    let mut client_a = TestClient::new();
    let mut client_b = TestClient::new();

    thread::sleep(Duration::from_millis(500)); // Allow clients to start

    // Step 1: Connect both clients and wait for initial sync
    println!("\n--- Step 1: Connecting clients ---");
    client_a.send_command("CONNECT 127.0.0.1:8000");
    client_b.send_command("CONNECT 127.0.0.1:8000");

    // Wait for connection confirmation
    client_a.wait_for_connection();
    client_b.wait_for_connection();

    // Wait for initial sync from both clients
    let initial_sync_a = client_a.wait_for_initial_sync();
    let initial_sync_b = client_b.wait_for_initial_sync();

    // Check initial states match
    assert_eq!(
        initial_sync_a, initial_sync_b,
        "Initial sync states don't match"
    );
    println!("✓ Initial sync states match");

    // Round 1: Client A sends "hello"
    println!("\n--- Round 1: Client A sends 'hello' ---");
    client_a.send_command("SEND hello");

    // Client A waits for OP_SENT confirmation (not SYNC)
    client_a.wait_for_op_sent();

    // Only Client B waits for SYNC (since Client A already knows about its own operation)
    let sync_b1 = client_b.wait_for_sync();

    println!("✓ Round 1 - Client B synced");

    // Round 2: Client B sends "hello world"
    println!("\n--- Round 2: Client B sends 'hello world' ---");
    client_b.send_command("SEND hello world");

    // Client B waits for OP_SENT confirmation
    client_b.wait_for_op_sent();

    // Only Client A waits for SYNC
    let sync_a2 = client_a.wait_for_sync();

    println!("✓ Round 2 - Client A synced");

    // Verify final states by having both clients check
    // (You might need to add a "GET_STATE" command to your client)
    println!("✓ Both rounds completed successfully");

    // Round 3: Exit both clients
    println!("\n--- Round 3: Shutting down ---");
    client_a.send_command("EXIT");
    client_b.send_command("EXIT");

    // Give clients time to exit
    thread::sleep(Duration::from_millis(100));

    println!("\n=== Test PASSED ===");
}
