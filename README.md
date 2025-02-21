# rust-server-monitor

A high-performance server monitoring tool written in Rust using Actix-web. It concurrently polls multiple frontend servers to collect system metrics (disk, CPU, memory) and provides a real-time dashboard for monitoring.

## Features

- **Concurrent Polling:** Uses asynchronous Rust features and stream combinators (e.g., `buffer_unordered`) to poll thousands of servers concurrently.
- **Real-Time Dashboard:** Displays detailed server metrics using a web dashboard built with Bootstrap 5.
- **Robust Error Handling:** Gracefully handles unresponsive or misbehaving servers.
- **Extensible:** Easily configurable to monitor additional metrics or integrate with other systems.

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (stable, with Cargo)
- [PM2](https://pm2.keymetrics.io/) (optional, for process management)
- A configured `frontends.json` file containing an array of server objects:
```json
  [
      { "name": "Server1", "ip": "192.168.1.100" },
      { "name": "Server2", "ip": "192.168.1.101" }
  ]
```

## Installation

```
# Clone the Repository:
git clone git@github.com:L0rdT33z/rust-server-monitor.git
cd rust-server-monitor

# Build the Project in Release Mode:
cargo build --release
```

## Running the Application

### Direct Execution

- **Using Cargo (for development):**
  
  ```bash
  cargo run
  ```

- **Running the Compiled Binary:**
  
  ```bash
  ./target/release/rust-server-monitor
  ```

### Running with PM2

PM2 is a process manager that can help you keep the application running continuously, including automatically restarting it on server startup.

    
0. **Install NodeJS & NPM:**
   
   ```bash
   sudo su -
   cd ~
   curl -sL https://deb.nodesource.com/setup_16.x -o /tmp/nodesource_setup.sh
   sudo bash /tmp/nodesource_setup.sh
   sudo apt install nodejs
   ```
    

1. **Install PM2 Globally:**
   
   ```bash
   npm install -g pm2
   ```

2. **Clone the Repository:**
```
# Clone the Repository:
   git clone git@github.com:L0rdT33z/rust-server-monitor.git
   cd rust-server-monitor

# Build the Project in Release Mode:
   cargo build --release
```


2. **Start Your Rust Application with PM2:**
   
   ```bash
   pm2 start ./target/release/rust-server-monitor --name rust-server-monitor
   ```

3. **Set Up PM2 to Run on System Startup:** PM2 provides a startup script that will configure your system to resurrect your processes on boot.
   
   ```bash
   pm2 startup
   ```
   
   This command will output a command that you need to run (typically with `sudo`). For example:
   
   ```bash
   sudo env PATH=$PATH:/usr/bin pm2 startup systemd -u youruser --hp /home/youruser
   ```
   
   After running the command, save the current process list:
   
   ```bash
   pm2 save
   ```

4. **Common PM2 Commands:**
   
   - **View Logs:**
     
     ```bash
     pm2 logs rust-server-monitor
     ```
   
   - **Stop the Process:**
     
     ```bash
     pm2 stop rust-server-monitor
     ```
   
   - **Restart the Process:**
     
     ```bash
     pm2 restart rust-server-monitor
     ```
   
   - **List All Processes:**
     
     ```bash
     pm2 list
     ```

## Configuration

- **Frontends File:**  
  The application expects a file named `frontends.json` in the root directory. This file should contain an array of frontend server definitions (name and IP) as shown above.

- **Polling Interval:**  
  The polling loop is currently set to run every 5 seconds. You can adjust this interval by modifying the `Duration::from_secs(5)` parameter in the source code.

## Contributing

Contributions are welcome! If you have suggestions, bug fixes, or new features, please open an issue or submit a pull request.

## License

This project is licensed under the [MIT License](https://chatgpt.com/c/LICENSE).

```
To use this file, simply create a new file named `README.md` in your project’s root directory, paste the above content, and save it.
```
