using System;
using System.Runtime.InteropServices;
using System.Threading;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Read
{
    /// <summary>Bounded retry for transient COM faults such as RPC_E_SERVERFAULT on the first read of a freshly attached session (frob:ticket ~4V111FE).</summary>
    public static class TransientRetry
    {
        /// <summary>RPC_E_SERVERFAULT: the server threw an exception while starting up.</summary>
        public const int ServerFault = unchecked((int)0x80010105);
        /// <summary>RPC_E_CALL_REJECTED: the server is busy and rejected the call.</summary>
        public const int CallRejected = unchecked((int)0x80010001);
        /// <summary>RPC_E_SERVERCALL_RETRYLATER: the server asks the caller to retry later.</summary>
        public const int RetryLater = unchecked((int)0x8001010A);

        /// <summary>Total attempts, including the first.</summary>
        public const int MaxAttempts = 4;

        /// <summary>True when the HRESULT names a fault that a short wait may clear.</summary>
        public static bool IsTransient(int hresult) =>
            hresult == ServerFault || hresult == CallRejected || hresult == RetryLater;

        /// <summary>Runs <paramref name="op"/>, retrying transient COM faults up to <see cref="MaxAttempts"/> times with linear backoff; other errors and the last failure propagate.</summary>
        public static T Run<T>(string what, Log log, Func<T> op, Action<TimeSpan>? sleep = null)
        {
            var wait = sleep ?? (d => Thread.Sleep(d));
            for (var attempt = 1; ; attempt++)
            {
                try
                {
                    return op();
                }
                catch (COMException e) when (IsTransient(e.ErrorCode) && attempt < MaxAttempts)
                {
                    var delay = TimeSpan.FromMilliseconds(500 * attempt);
                    log.Warn(what + ": transient COM fault 0x" + e.ErrorCode.ToString("X8") + " on attempt " + attempt + "; retrying in " + (int)delay.TotalMilliseconds + " ms");
                    wait(delay);
                }
            }
        }
    }
}
