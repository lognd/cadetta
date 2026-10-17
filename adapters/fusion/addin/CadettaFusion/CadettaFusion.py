"""In-Fusion add-in: a JSON-RPC server for the cadetta fusion adapter.

Fusion's Python runs only inside Fusion, so this add-in listens on
127.0.0.1:47360 (override with CADETTA_FUSION_ADDR) and answers the
newline-delimited JSON-RPC 2.0 requests of cadetta_fusion.session.RpcSession
(docs/spec/fusion.md 6). The API is main-thread only: the socket thread fires
a custom event and waits for the handler to fill in the reply.

doc.open also accepts a local .f3d/.f3z path (imported into a new document;
doc.save then exports back over that file) so live runs need no cloud id.

Status: see docs/spec/fusion.md 8. Standard library plus adsk only.
"""

import json
import logging
import os
import queue
import socket
import threading
import traceback

import adsk.core
import adsk.fusion

_logger = logging.getLogger("cadetta.fusion.addin")

LOCAL_SUFFIXES = (".f3d", ".f3z")
EVENT_ID = "CadettaFusionRpc"
ADDR = os.environ.get("CADETTA_FUSION_ADDR", "127.0.0.1:47360")

_app = None
_event = None
_server = None
_pending = queue.Queue()  # (request, reply_queue)
_opened = set()  # paths this add-in opened, so close only touches those
_local = {}  # local archive path -> Document imported from it


def _log(msg):
    """Write a line to Fusion's text command palette log and the module logger."""
    _logger.info(msg)
    if _app:
        _app.log("cadetta: " + msg)


def _is_local(path):
    """Tell whether `path` names an existing local Fusion archive (.f3d/.f3z)."""
    return path.lower().endswith(LOCAL_SUFFIXES) and os.path.isfile(path)


def _open_local(path):
    """Import the local archive at `path` into a new document and track it."""
    options = _app.importManager.createFusionArchiveImportOptions(path)
    doc = _app.importManager.importToNewDocument(options)
    _local[path] = doc
    _log("opened local archive " + path)
    return doc


def _save_local(path):
    """Export the imported document back over its source archive at `path`."""
    doc = _local[path]
    design = adsk.fusion.Design.cast(
        doc.products.itemByProductType("DesignProductType")
    )
    export = design.exportManager
    if not export.execute(export.createFusionArchiveExportOptions(path)):
        raise RuntimeError("local save failed: " + path)
    _log("saved local archive " + path)


def _design(path):
    """Return the open document at `path`, or raise KeyError."""
    if path in _local:
        return _local[path]
    for doc in _app.documents:
        if doc.dataFile is not None and doc.name == os.path.basename(path):
            return doc
        if getattr(doc, "fullName", "") == path:
            return doc
    raise KeyError(path)


def _doc_state(doc, attached):
    """Describe a document for doc.open."""
    return {
        "path": doc.name,
        "dirty": bool(doc.isModified),
        "readonly": bool(getattr(doc, "isReadOnly", False)),  # absent on some builds
        "attached": attached,
    }


def _params(doc):
    """Return the design's user parameters as dicts."""
    design = adsk.fusion.Design.cast(
        doc.products.itemByProductType("DesignProductType")
    )
    out = []
    for p in design.userParameters:
        out.append(
            {
                "name": p.name,
                "expression": p.expression,
                "unit": p.unit,
                "value": p.value,
                "comment": p.comment,
            }
        )
    return out


def _apply(doc, edits):
    """Set every expression; Fusion raises on a bad one, so restore all and re-raise."""
    design = adsk.fusion.Design.cast(
        doc.products.itemByProductType("DesignProductType")
    )
    params = design.userParameters
    old = {e["name"]: params.itemByName(e["name"]).expression for e in edits}
    try:
        for e in edits:
            params.itemByName(e["name"]).expression = e["expression"]
    except Exception:
        for name, expr in old.items():
            params.itemByName(name).expression = expr
        raise


def _handle(req):
    """Run one request on the main thread; return the JSON-RPC reply."""
    method, p = req["method"], req.get("params", {})
    try:
        if method == "fusion.version":
            result = _app.version
        elif method == "fusion.documents":
            result = [d.name for d in _app.documents]
            _log("open documents: %r" % (result,))
        elif method == "doc.open":
            path = p["path"]
            try:
                doc = _design(path)
                result = _doc_state(doc, True)
            except KeyError:
                if _is_local(path):
                    doc = _open_local(path)
                else:
                    doc = _app.documents.open(_app.data.findFileById(path), False)
                _opened.add(path)
                result = _doc_state(doc, False)
        elif method == "doc.parameters":
            result = _params(_design(p["path"]))
        elif method == "doc.apply":
            _apply(_design(p["path"]), p["edits"])
            result = None
        elif method == "doc.save":
            if p["path"] in _local:
                _save_local(p["path"])
            elif not _design(p["path"]).save("cadetta push"):
                raise RuntimeError("save failed")
            result = None
        elif method == "doc.close":
            if p["path"] in _opened:
                _design(p["path"]).close(False)
                _opened.discard(p["path"])
                _local.pop(p["path"], None)
                _log("closed " + p["path"])
            result = None
        else:
            return {
                "jsonrpc": "2.0",
                "id": req["id"],
                "error": {"code": -32601, "message": method},
            }
        return {"jsonrpc": "2.0", "id": req["id"], "result": result}
    except Exception as e:  # reported to the adapter as an error reply
        _log(traceback.format_exc())
        return {
            "jsonrpc": "2.0",
            "id": req["id"],
            "error": {"code": -32000, "message": str(e)},
        }


class _Handler(adsk.core.CustomEventHandler):
    """Drains queued requests on Fusion's main thread."""

    def notify(self, args):
        """Answer every queued request."""
        while True:
            try:
                req, reply = _pending.get_nowait()
            except queue.Empty:
                return
            reply.put(_handle(req))


def _serve_conn(conn):
    """Read requests from one adapter connection until it closes."""
    f = conn.makefile("rw", encoding="utf-8", newline="\n")
    for line in f:
        reply = queue.Queue()
        _pending.put((json.loads(line), reply))
        _app.fireCustomEvent(EVENT_ID)
        f.write(json.dumps(reply.get()) + "\n")
        f.flush()
    conn.close()


def _accept():
    """Accept adapter connections, one thread each."""
    while True:
        try:
            conn, _ = _server.accept()
        except OSError:
            return
        threading.Thread(target=_serve_conn, args=(conn,), daemon=True).start()


def run(context):
    """Fusion entry point: register the event and start listening."""
    global _app, _event, _server
    _app = adsk.core.Application.get()
    _event = _app.registerCustomEvent(EVENT_ID)
    _handler = _Handler()
    _event.add(_handler)
    run._keep = _handler
    host, _, port = ADDR.rpartition(":")
    _server = socket.socket()
    _server.bind((host, int(port)))
    _server.listen(1)
    threading.Thread(target=_accept, daemon=True).start()
    _log("listening on " + ADDR)


def stop(context):
    """Fusion exit point: stop listening and unregister the event."""
    if _server:
        _server.close()
    if _app:
        _app.unregisterCustomEvent(EVENT_ID)
