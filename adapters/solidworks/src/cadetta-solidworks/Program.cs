using System;
using System.IO;
using System.Text;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Server;
using Cadetta.SolidWorks.Session;

namespace Cadetta.SolidWorks.Cli
{
    /// <summary>The cadetta-solidworks protocol executable.</summary>
    public static class Program
    {
        /// <summary>Serves JSON-RPC on stdio (stdout is protocol only), or prints the version.</summary>
        public static int Main(string[] args)
        {
            var utf8 = new UTF8Encoding(false);
            var stderr = new StreamWriter(Console.OpenStandardError(), utf8) { NewLine = "\n", AutoFlush = true };
            var log = new Log(stderr, Log.ParseFilter(Environment.GetEnvironmentVariable("CADETTA_LOG")));

            if (Array.IndexOf(args, "--version") >= 0)
            {
                // Intentional command output, not logging.
                Console.Out.Write("cadetta-solidworks " + AdapterInfo.Version + "\n");
                return 0;
            }

#if CADETTA_COM
            Cadetta.SolidWorks.Com.Session.InteropResolver.Install(log);
#endif
            var probeAt = Array.IndexOf(args, "--probe-api");
            if (probeAt >= 0)
            {
                return RunProbe(args, probeAt, log);
            }

            var fixAt = Array.IndexOf(args, "--make-read-fixture");
            if (fixAt >= 0 && fixAt + 1 < args.Length)
            {
                return RunMakeFixture(args[fixAt + 1], log);
            }

            var readAt = Array.IndexOf(args, "--read-doc");
            if (readAt >= 0 && readAt + 2 < args.Length)
            {
                return RunReadDoc(args[readAt + 1], args[readAt + 2], log);
            }

            var fake = Array.IndexOf(args, "--fake") >= 0
                || Environment.GetEnvironmentVariable("CADETTA_SW_FAKE") == "1";
            ISwSession session;
            IDisposable? owner = null;
            IPushHost? pushHost = null;
            Cadetta.SolidWorks.Regen.IRegenHost? regenHost = null;
            Cadetta.SolidWorks.Read.ReadService? readService = null;
            if (fake)
            {
                log.Info("using the fake session");
                session = new FakeSwSession();
            }
            else
            {
#if CADETTA_COM
                log.Info("connecting to SOLIDWORKS (attach or start hidden)");
                var com = Cadetta.SolidWorks.Com.SwComSession.Attach(log);
                owner = com.Connection;
                session = com;
                pushHost = com.PushHost(log);
                regenHost = com.RegenHost(log);
                readService = com.ReadService(log);
#else
                log.Error("built without the COM session; pass --fake");
                return 2;
#endif
            }

            var input = new StreamReader(Console.OpenStandardInput(), utf8);
            var output = new StreamWriter(Console.OpenStandardOutput(), utf8) { NewLine = "\n", AutoFlush = true };
            try
            {
                return new RpcServer(session, log, readService, pushHost, regenHost).Run(input, output);
            }
            finally
            {
                // Quits SOLIDWORKS only if this process started it.
                owner?.Dispose();
            }
        }

        /// <summary>Authors the read-verification fixture from a source part (frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4).</summary>
        private static int RunMakeFixture(string sourcePart, Log log)
        {
#if CADETTA_COM
            var com = Cadetta.SolidWorks.Com.SwComSession.Attach(log);
            try
            {
                log.Info("fixture assembly: " + com.MakeReadFixture(sourcePart, log));
                return 0;
            }
            finally
            {
                com.Connection?.Dispose();
            }
#else
            log.Error("built without the COM session; --make-read-fixture needs SOLIDWORKS");
            return 2;
#endif
        }

        /// <summary>Reads one document to observed IR JSON for verification (frob:ticket 01M4SV2TKZD26CK9FFA1EJXSV4).</summary>
        private static int RunReadDoc(string file, string outPath, Log log)
        {
#if CADETTA_COM
            var com = Cadetta.SolidWorks.Com.SwComSession.Attach(log);
            try
            {
                var r = com.ReadObserved(file, log);
                File.WriteAllText(outPath, System.Text.Json.JsonSerializer.Serialize(r, new System.Text.Json.JsonSerializerOptions { WriteIndented = true }));
                return 0;
            }
            finally
            {
                com.Connection?.Dispose();
            }
#else
            log.Error("built without the COM session; --read-doc needs SOLIDWORKS");
            return 2;
#endif
        }

        /// <summary>Runs the SOLIDWORKS API probe and prints its markdown table (intentional command output).</summary>
        private static int RunProbe(string[] args, int at, Log log)
        {
#if CADETTA_COM
            var scratch = at + 1 < args.Length ? args[at + 1] : Path.Combine(Path.GetTempPath(), "cadetta-api-probe");
            log.Info("probing the SOLIDWORKS API surface in " + scratch);
            var com = Cadetta.SolidWorks.Com.SwComSession.Attach(log);
            try
            {
                var results = com.ApiProbe(log).Run(scratch);
                var md = Cadetta.SolidWorks.Probe.ApiProbeReport.ToMarkdown(results);
                Console.Out.Write(md);
                File.WriteAllText(Path.Combine(scratch, "probe-result.md"), md);
                return Cadetta.SolidWorks.Probe.ApiProbeReport.AllOk(results) ? 0 : 1;
            }
            finally
            {
                com.Connection?.Dispose();
            }
#else
            log.Error("built without the COM session; --probe-api needs SOLIDWORKS");
            return 2;
#endif
        }
    }
}
