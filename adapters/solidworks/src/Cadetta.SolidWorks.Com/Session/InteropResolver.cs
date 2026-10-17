using System;
using System.IO;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Com.Session
{
    // frob:ticket ~Z9SWC8W
    /// <summary>
    /// Resolves SolidWorks.Interop.* assemblies from SOLIDWORKS_API_DIR at run
    /// time. The interop assemblies are referenced but never copied next to the
    /// executable (Interop.props sets Private=false), so they are loaded on demand.
    /// </summary>
    public static class InteropResolver
    {
        private const string Prefix = "SolidWorks.Interop.";

        /// <summary>Installs the resolver; call before any SOLIDWORKS type is used.</summary>
        public static void Install(Log log)
        {
            var dir = Environment.GetEnvironmentVariable("SOLIDWORKS_API_DIR");
            if (string.IsNullOrEmpty(dir))
            {
                dir = @"C:\Program Files\SOLIDWORKS Corp\SOLIDWORKS\api\redist";
            }
            log.Info("resolving SolidWorks.Interop assemblies from " + dir);
            AppDomain.CurrentDomain.AssemblyResolve += (_, args) =>
            {
                var name = new System.Reflection.AssemblyName(args.Name).Name;
                if (name == null || !name.StartsWith(Prefix, StringComparison.Ordinal))
                {
                    return null;
                }
                var path = Path.Combine(dir, name + ".dll");
                if (!File.Exists(path))
                {
                    log.Error("interop assembly not found: " + path);
                    return null;
                }
                log.Debug("loading " + path);
                return System.Reflection.Assembly.LoadFrom(path);
            };
        }
    }
}
