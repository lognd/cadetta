using System;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Session
{
    /// <summary>
    /// A live application handle that remembers whether this adapter started it,
    /// so disposing closes only what the adapter opened (adapters.md 2.2).
    /// </summary>
    /// <typeparam name="T">The application handle type.</typeparam>
    public sealed class SwConnection<T> : IDisposable where T : class
    {
        private readonly ISwHost<T> _host;
        private readonly Log _log;
        private bool _disposed;

        private SwConnection(ISwHost<T> host, T app, bool started, Log log)
        {
            _host = host;
            App = app;
            Started = started;
            _log = log;
        }

        /// <summary>The application handle.</summary>
        public T App { get; }

        /// <summary>True when this adapter started the instance (false when it attached).</summary>
        public bool Started { get; }

        /// <summary>Attaches to a running instance if one exists, otherwise starts one hidden.</summary>
        public static SwConnection<T> Connect(ISwHost<T> host, Log log)
        {
            var running = host.TryAttach();
            if (running != null)
            {
                log.Info("attached to the running SOLIDWORKS");
                return new SwConnection<T>(host, running, false, log);
            }
            log.Info("no running SOLIDWORKS; starting one hidden");
            var started = host.StartHidden();
            return new SwConnection<T>(host, started, true, log);
        }

        /// <summary>Quits the application only if this adapter started it; attached instances are left running.</summary>
        public void Dispose()
        {
            if (_disposed)
            {
                return;
            }
            _disposed = true;
            if (Started)
            {
                _log.Info("quitting the SOLIDWORKS this adapter started");
                _host.Quit(App);
            }
            else
            {
                _log.Debug("leaving the attached SOLIDWORKS running");
            }
        }
    }
}
