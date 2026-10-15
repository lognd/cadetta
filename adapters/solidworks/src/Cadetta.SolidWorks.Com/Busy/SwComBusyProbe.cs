using System;
using System.Runtime.InteropServices;
using Cadetta.SolidWorks.Busy;
using Cadetta.SolidWorks.Logging;
using SolidWorks.Interop.sldworks;

namespace Cadetta.SolidWorks.Com.Busy
{
    /// <summary>IBusyProbe over COM: CommandInProgress for commands, a disabled main window for modal dialogs.</summary>
    public sealed class SwComBusyProbe : IBusyProbe
    {
        private readonly ISldWorks _sw;
        private readonly Log _log;

        /// <summary>Creates a probe over <paramref name="sw"/>.</summary>
        public SwComBusyProbe(ISldWorks sw, Log log)
        {
            _sw = sw;
            _log = log;
        }

        [DllImport("user32.dll")]
        private static extern bool IsWindowEnabled(IntPtr hWnd);

        /// <summary>A modal dialog disables the SOLIDWORKS main window.</summary>
        public bool ModalDialogOpen()
        {
            var frame = (IFrame)_sw.Frame();
            var hwnd = new IntPtr(frame.GetHWnd());
            var modal = hwnd != IntPtr.Zero && !IsWindowEnabled(hwnd);
            _log.Debug("main window disabled (modal): " + modal);
            return modal;
        }

        /// <summary>True while SOLIDWORKS reports a command in progress.</summary>
        public bool CommandActive()
        {
            var active = _sw.CommandInProgress;
            _log.Debug("command in progress: " + active);
            return active;
        }
    }
}
