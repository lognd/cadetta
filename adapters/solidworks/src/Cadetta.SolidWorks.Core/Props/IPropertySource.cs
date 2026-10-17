using System.Collections.Generic;
using Cadetta.SolidWorks.Read;

namespace Cadetta.SolidWorks.Props
{
    /// <summary>
    /// The COM edge of the configuration and custom-property read: names the
    /// configurations and yields one property set at a time.
    /// frob:ticket 01M4SWME29Y61DC7AKZD3HBZSH
    /// </summary>
    public interface IPropertySource
    {
        /// <summary>The active configuration's name; null when the document has none (a drawing).</summary>
        string? ActiveConfiguration { get; }

        /// <summary>Every configuration name of the document, active one included.</summary>
        IReadOnlyList<string> Configurations { get; }

        /// <summary>The custom properties of <paramref name="config"/>, or of the file level when it is null.</summary>
        IReadOnlyList<NativeProperty> Read(string? config);
    }

    /// <summary>In-memory <see cref="IPropertySource"/> for tests.</summary>
    public sealed class FakePropertySource : IPropertySource
    {
        private readonly Dictionary<string, List<NativeProperty>> _sets = new Dictionary<string, List<NativeProperty>>();

        /// <summary>The active configuration name.</summary>
        public string? ActiveConfiguration { get; set; }

        /// <summary>The configuration names.</summary>
        public List<string> Names { get; } = new List<string>();

        /// <summary>Configuration names as the interface exposes them.</summary>
        public IReadOnlyList<string> Configurations => Names;

        /// <summary>Registers a property under <paramref name="config"/> (null for file level).</summary>
        public FakePropertySource With(string? config, string name, string value, string? expression = null)
        {
            var key = config ?? "";
            if (!_sets.TryGetValue(key, out var list))
            {
                _sets[key] = list = new List<NativeProperty>();
            }
            list.Add(new NativeProperty { Name = name, Config = config, Value = value, Expression = expression });
            return this;
        }

        /// <summary>Returns the registered set, empty when none.</summary>
        public IReadOnlyList<NativeProperty> Read(string? config) =>
            _sets.TryGetValue(config ?? "", out var list) ? list : new List<NativeProperty>();
    }
}
