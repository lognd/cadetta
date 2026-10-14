using System;
using System.Globalization;
using System.IO;

namespace Cadetta.SolidWorks.Logging
{
    /// <summary>Log severity, ordered from most to least severe.</summary>
    public enum LogLevel
    {
        /// <summary>Failures.</summary>
        Error = 0,
        /// <summary>Recoverable problems.</summary>
        Warn = 1,
        /// <summary>State changes and boundaries.</summary>
        Info = 2,
        /// <summary>Branches and decisions.</summary>
        Debug = 3,
        /// <summary>Everything.</summary>
        Trace = 4,
    }

    /// <summary>
    /// Stderr logger in the shape of Rust tracing output, filtered by the
    /// CADETTA_LOG directive (a bare level, or comma-separated target=level
    /// pairs where only the cadetta_solidworks target and bare levels count).
    /// </summary>
    public sealed class Log
    {
        private const string Target = "cadetta_solidworks";
        private readonly TextWriter _sink;
        private readonly LogLevel _max;

        /// <summary>Creates a logger writing to <paramref name="sink"/> at most as verbose as <paramref name="max"/>.</summary>
        public Log(TextWriter sink, LogLevel max)
        {
            _sink = sink;
            _max = max;
        }

        /// <summary>A logger that discards everything.</summary>
        public static Log Null() => new Log(TextWriter.Null, LogLevel.Error);

        /// <summary>Parses a CADETTA_LOG filter; null or unparsable gives warn.</summary>
        public static LogLevel ParseFilter(string? directives)
        {
            var level = LogLevel.Warn;
            if (string.IsNullOrWhiteSpace(directives))
            {
                return level;
            }
            foreach (var raw in directives!.Split(','))
            {
                var d = raw.Trim();
                var eq = d.IndexOf('=');
                var target = eq < 0 ? "" : d.Substring(0, eq).Trim();
                var name = eq < 0 ? d : d.Substring(eq + 1).Trim();
                if (target.Length != 0 && target != Target && target != "cadetta")
                {
                    continue;
                }
                if (TryLevel(name, out var parsed))
                {
                    level = parsed;
                }
            }
            return level;
        }

        private static bool TryLevel(string name, out LogLevel level)
        {
            switch (name.ToLowerInvariant())
            {
                case "error": level = LogLevel.Error; return true;
                case "warn": level = LogLevel.Warn; return true;
                case "info": level = LogLevel.Info; return true;
                case "debug": level = LogLevel.Debug; return true;
                case "trace": level = LogLevel.Trace; return true;
                default: level = LogLevel.Warn; return false;
            }
        }

        /// <summary>True when records at <paramref name="level"/> are written.</summary>
        public bool Enabled(LogLevel level) => level <= _max;

        /// <summary>Writes one record if the level is enabled.</summary>
        public void Write(LogLevel level, string message)
        {
            if (!Enabled(level))
            {
                return;
            }
            var ts = DateTime.UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss.ffffff'Z'", CultureInfo.InvariantCulture);
            var lv = level.ToString().ToUpperInvariant().PadLeft(5);
            _sink.Write(ts + " " + lv + " " + Target + ": " + message + "\n");
            _sink.Flush();
        }

        /// <summary>Logs at error.</summary>
        public void Error(string m) => Write(LogLevel.Error, m);
        /// <summary>Logs at warn.</summary>
        public void Warn(string m) => Write(LogLevel.Warn, m);
        /// <summary>Logs at info.</summary>
        public void Info(string m) => Write(LogLevel.Info, m);
        /// <summary>Logs at debug.</summary>
        public void Debug(string m) => Write(LogLevel.Debug, m);
    }
}
