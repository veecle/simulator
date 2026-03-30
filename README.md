# Cruise Control Simulator Bridge

A Rust-based bridge that connects the veecle-os runtime with an external vehicle simulation system.
It enables real-time control and sensor data exchange through a Unix socket using a JSON protocol.

## Overview

This simulator acts as an interface layer that:
- Receives vehicle control commands (throttle, brake, steering) from veecle-os
- Forwards these commands to an external simulation via Unix socket
- Receives sensor data (primarily speed) from the simulation
- Publishes sensor readings back to the veecle-os runtime

## Architecture

```
veecle-os Runtime <---> Simulator Bridge <---> External Simulation
                         (this project)         (/opt/simulation.sock)
```

### Communication Protocol

The bridge communicates with the external simulation using:
- **Transport**: Unix domain socket at `/opt/simulation.sock`
- **Format**: Newline-delimited JSON
- **Direction**: Bidirectional (control commands out, sensor data in)

## Data Types

### Control Commands (veecle-os → Simulation)

- **Throttle**: Position control (0.0 = closed, 1.0 = fully open)
- **Brake**: Pressure control (0.0 = released, 1.0 = maximum pressure)
- **Steering**: Angle control (-1.0 = full left, 0.0 = center, 1.0 = full right)
- **ParkingBrake**: Engagement control (0.0 = released, 1.0 = fully engaged)

### Sensor Data (Simulation → veecle-os)

- **Speed**: Vehicle speed in kilometers per hour (km/h)

