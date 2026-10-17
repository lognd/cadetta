// frob:ticket ~NEHY9X7
using System;
using System.IO;
using System.Reflection;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.AddIn
{
    /// <summary>
    /// Resolves the add-in's own dependencies by simple name from the add-in's folder. SLDWORKS.exe has no
    /// binding redirects, so a reference such as System.Runtime.CompilerServices.Unsafe 4.0.4.1 (wanted by
    /// System.Text.Json) would otherwise fail while the folder holds a newer build of it.
    /// </summary>
    public static class AddInDependencyResolver
    {
        private static bool _installed;

        /// <summary>Installs the resolver once; call before any dependency of the add-in is used.</summary>
        public static void Install(Log log)
        {
            if (_installed)
            {
                return;
            }
            _installed = true;
            var dir = Path.GetDirectoryName(typeof(AddInDependencyResolver).Assembly.Location) ?? "";
            log.Info("add-in: resolving dependencies from " + dir);
            AppDomain.CurrentDomain.AssemblyResolve += (_, args) =>
            {
                var name = new AssemblyName(args.Name).Name;
                var path = Path.Combine(dir, name + ".dll");
                if (name == null || name.StartsWith("SolidWorks.Interop.", StringComparison.Ordinal) || !File.Exists(path))
                {
                    return null;
                }
                log.Debug("add-in: resolved " + args.Name + " to " + path);
                return System.Reflection.Assembly.LoadFrom(path);
            };
        }
    }
}
