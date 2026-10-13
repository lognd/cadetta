"""Required airflow and system curve for the cooling module (cadetta.solve/1, stdio).

Fan-independent: Q = margin * P / (rho * cp * dT); k comes from the enclosure table.
"""
import csv
import json
import sys
from pathlib import Path

HERE = Path(__file__).parent


def table(name: str) -> dict[str, float]:
    """Read a quantity,value,unit CSV into a name -> value map."""
    with open(HERE / name, newline="") as fh:
        return {row["quantity"]: float(row["value"]) for row in csv.DictReader(fh)}


def main() -> None:
    """Answer one cadetta.solve/1 request on stdin with the outputs on stdout."""
    req = json.load(sys.stdin)
    inputs = req["inputs"]
    air = table("air_properties.csv")
    enc = table("enclosure_impedance.csv")
    power_w = inputs["heat_load"]["si"]
    dt_k = inputs["dt_max"]["si"]
    margin = inputs["margin"]["si"]
    flow_m3h = margin * power_w / (air["density"] * air["specific_heat"] * dt_k) * 3600
    out = {
        "schema": "cadetta.solve/1",
        "outputs": {
            "required_flow": f"{flow_m3h:.3g}m^3/h",
            "system_k": f"{enc['system_k']:.6g}Pa*s^2/m^6",
        },
    }
    json.dump(out, sys.stdout)


if __name__ == "__main__":
    main()
