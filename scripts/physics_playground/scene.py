"""The scene being built: entities in authored order, with helpers for the
kinds of thing the playground is made of."""

from __future__ import annotations

from common import (
    body, box, capsule, circle, rgba, script, shape, transform,
)

PARTS = "scripts/parts.decay"

# Room palette.
STATIC_FILL = "#1b2838"
STATIC_EDGE = "#4a6a86"


class Scene:
    def __init__(self) -> None:
        self.entities: list[dict] = []
        self.ids: set[str] = set()

    def add(self, entity_id, name=None, x=0.0, y=0.0, z=0.0, sx=1.0, sy=None, rot=0.0,
            parent=None, components=None, tags=None):
        if entity_id in self.ids:
            raise ValueError(f"duplicate entity id {entity_id}")
        self.ids.add(entity_id)
        entity = {"id": entity_id, "name": name or entity_id}
        if parent:
            entity["parent"] = parent
        entity["transform_3d"] = transform(x, y, z, sx, sy, rot)
        entity["components"] = dict(components or {})
        if tags:
            entity["components"]["sindri.tags"] = {"tags": list(tags)}
        self.entities.append(entity)
        return entity

    # Static geometry: a drawn, colliding box.
    def wall(self, entity_id, x, y, w, h, rot=0.0, fill=STATIC_FILL, edge=STATIC_EDGE,
             friction=0.6, restitution=0.0, layer=2, extra=None, name=None):
        components = {
            "sindri.shape": shape("rect", fill, edge, sw=min(0.08, 0.06 / max(0.2, min(w, h))),
                                  layer=layer),
            "sindri.physics2d.collider": {"pieces": [
                box(w / 2, h / 2, "static", friction, restitution)]},
        }
        components.update(extra or {})
        return self.add(entity_id, name, x, y, 0.0, w, h, rot, components=components)

    # A dynamic box that a reset puts back where it started.
    def crate(self, entity_id, x, y, w, h, toy, fill, edge=None, rot=0.0, friction=0.5,
              restitution=0.05, layer="props", mat=None, damping=0.05, name=None,
              ccd=False, tags=("body",), z=0.2):
        components = {
            "sindri.shape": shape("rect", fill, edge, sw=0.06 if edge else 0.0,
                                  layer=5),
            "sindri.physics2d.collider": {"pieces": [
                box(w / 2, h / 2, layer, friction, restitution)]},
            "sindri.physics2d.rigid_body": body(damping=damping, angular_damping=damping,
                                                ccd=ccd),
            "sindri.script": script(PARTS, "Part", toy=float(toy)),
        }
        if mat:
            components["sindri.physics2d.material"] = mat
        return self.add(entity_id, name, x, y, z, w, h, rot, components=components, tags=tags)

    def ball(self, entity_id, x, y, radius, toy, fill, edge=None, friction=0.4,
             restitution=0.5, layer="balls", mat=None, name=None, ccd=False,
             tags=("body",), z=0.25):
        components = {
            "sindri.shape": shape("ellipse", fill, edge,
                                  sw=0.08 if edge else 0.0, layer=6),
            "sindri.physics2d.collider": {"pieces": [
                circle(radius, layer, friction, restitution)]},
            "sindri.physics2d.rigid_body": body(damping=0.02, angular_damping=0.05, ccd=ccd),
            "sindri.script": script(PARTS, "Part", toy=float(toy)),
        }
        if mat:
            components["sindri.physics2d.material"] = mat
        return self.add(entity_id, name, x, y, z, radius * 2, radius * 2,
                        components=components, tags=tags)

    # An invisible owner of one authored joint.
    def joint(self, entity_id, kind, payload, toy, name=None, behaviour=None):
        components = {f"sindri.physics2d.{kind}_joint": payload}
        components["sindri.script"] = behaviour or script(PARTS, "JointPart", toy=float(toy))
        return self.add(entity_id, name, components=components, tags=("joint",))

    # A purely drawn shape.
    def deco(self, entity_id, x, y, w, h, kind="rect", fill="#ffffff", alpha=1.0,
             edge=None, sw=0.0, layer=1, rot=0.0, parent=None, count=6, z=0.0,
             name=None, dashes=0, blend="over", components=None):
        merged = {"sindri.shape": shape(kind, fill, edge, sw=sw, layer=layer, count=count,
                                        alpha=alpha, dashes=dashes, blend=blend)}
        merged.update(components or {})
        return self.add(entity_id, name, x, y, z, w, h, rot, parent=parent,
                        components=merged)


def hinge(first, second, first_anchor=(0, 0), second_anchor=(0, 0), limits=None,
          motor=None):
    payload = {
        "first": first, "second": second,
        "first_anchor": list(first_anchor), "second_anchor": list(second_anchor),
        "limits_enabled": limits is not None,
        "lower_angle": limits[0] if limits else 0.0,
        "upper_angle": limits[1] if limits else 0.0,
        "motor_enabled": False, "motor_mode": "velocity", "motor_velocity": 0.0,
        "motor_max_torque": 0.0, "motor_target_angle": 0.0, "motor_stiffness": 0.0,
        "motor_damping": 0.0,
    }
    if motor:
        payload.update(motor)
        payload["motor_enabled"] = True
    return payload


def slider(first, second, axis=(1, 0), first_anchor=(0, 0), second_anchor=(0, 0),
           limits=None, motor=None):
    payload = {
        "first": first, "second": second,
        "first_anchor": list(first_anchor), "second_anchor": list(second_anchor),
        "first_axis": list(axis), "second_axis": list(axis),
        "limits_enabled": limits is not None,
        "lower_distance": limits[0] if limits else 0.0,
        "upper_distance": limits[1] if limits else 0.0,
        "motor_enabled": False, "motor_mode": "velocity", "motor_velocity": 0.0,
        "motor_max_force": 0.0, "motor_target_distance": 0.0, "motor_stiffness": 0.0,
        "motor_damping": 0.0,
    }
    if motor:
        payload.update(motor)
        payload["motor_enabled"] = True
    return payload


def spring(first, second, rest, stiffness, damping, first_anchor=(0, 0),
           second_anchor=(0, 0)):
    return {
        "first": first, "second": second,
        "first_anchor": list(first_anchor), "second_anchor": list(second_anchor),
        "rest_length": rest, "stiffness": stiffness, "damping": damping,
    }


def distance(first, second, length):
    return {"first": first, "second": second, "max_distance": length}


__all__ = ["Scene", "hinge", "slider", "spring", "distance", "rgba", "capsule", "circle",
           "box", "body", "script", "shape"]
