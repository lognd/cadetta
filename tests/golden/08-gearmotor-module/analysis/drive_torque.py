"""Per-motor wheel torque for a 2-motor robot on a slope (cadetta.solve/1 over stdio)."""
import logging
import math

from cadetta import quantity
from cadetta.solve import solver

_log = logging.getLogger(__name__)

G = 9.80665     # m/s^2, standard gravity
ACCEL = 0.5     # m/s^2, REQ-12 s3.1
SLOPE = 10.0    # deg, REQ-12 s3.2
CRR = 0.02      # rolling resistance, assumed


@solver(inputs={"robot_mass": "Mass", "wheel_radius": "Length"},
        outputs={"torque_per_motor": "Torque"}, significant=6)
def drive_torque(robot_mass, wheel_radius):
    """Return the torque each of two driven wheels must supply: accelerate up the slope."""
    mass = robot_mass.to_float("kg")
    radius = wheel_radius.to_float("m")
    slope = math.radians(SLOPE)
    force = mass * (ACCEL + G * math.sin(slope) + G * CRR * math.cos(slope))
    torque = force / 2 * radius
    _log.info("mass=%s kg radius=%s m torque=%s N*m", mass, radius, torque)
    return {"torque_per_motor": quantity(f"{torque:.9g}N*m").unwrap()}


if __name__ == "__main__":
    drive_torque.serve()
