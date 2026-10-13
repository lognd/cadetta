"""Per-motor wheel torque for a 2-motor robot on a slope (stdio solver, cadetta.solve/1)."""
import json
import math
import sys

G = 9.80665
ACCEL = 0.5     # m/s^2, REQ-12 s3.1
SLOPE = 10.0    # deg, REQ-12 s3.2
CRR = 0.02      # rolling resistance, assumed

req = json.load(sys.stdin)
mass = req["inputs"]["robot_mass"]["si"]
radius = req["inputs"]["wheel_radius"]["si"]
force = mass * (ACCEL + G * math.sin(math.radians(SLOPE)) + G * CRR * math.cos(math.radians(SLOPE)))
torque = force / 2 * radius
json.dump({"schema": "cadetta.solve/1",
           "outputs": {"torque_per_motor": "%.3fN*m" % torque},
           "diagnostics": []}, sys.stdout)
