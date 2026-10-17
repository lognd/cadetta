// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
using System;
using System.Collections.Generic;
using System.Linq;
using Cadetta.SolidWorks.AddIn.Client;

namespace Cadetta.SolidWorks.AddIn.Pane
{
    /// <summary>The dimension the engineer selected: its kind and value in SI (solidworks.md 2.1).</summary>
    public sealed class SelectedDimension
    {
        /// <summary>Creates a selection.</summary>
        public SelectedDimension(QuantityKind kind, double si)
        {
            Kind = kind;
            Si = si;
        }

        /// <summary>Kind of the dimension.</summary>
        public QuantityKind Kind { get; }

        /// <summary>Value in SI.</summary>
        public double Si { get; }
    }

    /// <summary>Pure row selection for the pane: the document component's paths, narrowed by a selected dimension.</summary>
    public static class PathFilter
    {
        /// <summary>The component a document stands for: its file name without extension.</summary>
        public static string ComponentOf(string documentPath)
        {
            var name = documentPath.Replace('\\', '/');
            name = name.Substring(name.LastIndexOf('/') + 1);
            var dot = name.LastIndexOf('.');
            return dot > 0 ? name.Substring(0, dot) : name;
        }

        /// <summary>The leading component segment of a canonical path (`pkg::Bracket.holes.pitch_x` gives `Bracket`).</summary>
        public static string ComponentSegment(string path)
        {
            var colons = path.LastIndexOf("::", StringComparison.Ordinal);
            var rest = colons < 0 ? path : path.Substring(colons + 2);
            var end = rest.IndexOfAny(new[] { '.', '/' });
            return end < 0 ? rest : rest.Substring(0, end);
        }

        /// <summary>Rows belonging to <paramref name="component"/>, in the CLI's (path) order.</summary>
        public static IReadOnlyList<NameRow> ForComponent(IEnumerable<NameRow> rows, string component) =>
            rows.Where(r => string.Equals(ComponentSegment(r.Path), component, StringComparison.OrdinalIgnoreCase)).ToList();

        /// <summary>Params of the selection's kind, nearest value first (ties keep path order); all rows when nothing is selected.</summary>
        public static IReadOnlyList<NameRow> ForSelection(IReadOnlyList<NameRow> rows, SelectedDimension? selection)
        {
            if (selection == null)
            {
                return rows;
            }
            return rows
                .Where(r => r.Kind == "param")
                .Select((r, i) => (Row: r, Index: i, Q: Quantity.Parse(r.Emit)))
                .Where(x => x.Q.Kind == selection.Kind)
                .OrderBy(x => Math.Abs(x.Q.Si - selection.Si))
                .ThenBy(x => x.Index)
                .Select(x => x.Row)
                .ToList();
        }
    }
}
