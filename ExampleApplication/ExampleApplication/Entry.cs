using Foundation;
using Native;
using Script;
using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace ExampleApplication
{
    using RuntimeApplicationType = IntPtr;
    using NativeDeviceType = IntPtr;

    public static unsafe class Entry
    {
        public static Script.ScriptEngine ScriptEngine;
        public static Application Application = new();

        [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
        public static void Main(NativeEntryInfo* nativeEntryInfo)
        {
            UnmanagedObject<Application> unmanagedApplication = new(Application); // FIXME: Application is never free

            nativeEntryInfo->runtimeApplication = unmanagedApplication.GetHandlePointer();  // Util.ToUnmanagedPtr(application);
            *nativeEntryInfo->nativeRuntimeApplicationFunctions = new NativeApplicationFunctions();
            *nativeEntryInfo->fileWatchFunctions = new FileWatchFunctions();
            NativeDevice.Functions = nativeEntryInfo->nativeGpuContextFunctions;
            NativeDevice.nativeDevice = nativeEntryInfo->nativeDevice;
            NativeCommandEncoder.Functions = nativeEntryInfo->nativeCommandEncoderFunctions;
            NativeRenderPass.Functions = nativeEntryInfo->nativeRenderPassFunctions;
            NativeQueue.Functions = nativeEntryInfo->nativeQueueFunctions;
            NativeShaderModule.Functions = nativeEntryInfo->nativeShaderModuleFunctions;
            NativePipelineLayout.Functions = nativeEntryInfo->nativePipelineLayoutFunctions;
            NativeRenderPipeline.Functions = nativeEntryInfo->nativeRenderPipelineFunctions;
            //System.Diagnostics.Debugger.Launch();
            ScriptEngine = new Script.ScriptEngine();
            ScriptEngine.Reload();
            Application.userSscript = ScriptEngine.userSscript;
            Application.Initialize();

            Console.WriteLine(".NET C# Engine is running.");
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct NativeEntryInfo
    {
        public RuntimeApplicationType runtimeApplication;
        public NativeApplicationFunctions* nativeRuntimeApplicationFunctions;
        public FileWatchFunctions* fileWatchFunctions;
        public NativeDeviceFunctions nativeGpuContextFunctions;
        public NativeDeviceType nativeDevice;
        public NativeCommandEncoderFunctions nativeCommandEncoderFunctions;
        public NativeRenderPassFunctions nativeRenderPassFunctions;
        public NativeQueueFunctions nativeQueueFunctions;
        public NativeShaderModuleFunctions nativeShaderModuleFunctions;
        public NativeRenderPipelineFunctions nativeRenderPipelineFunctions;
        public NativePipelineLayoutFunctions nativePipelineLayoutFunctions;
    }

    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct FileWatchFunctions
    {
        public unsafe delegate* unmanaged<void> runtimeSourceFileChanged = &SourceFileChanged;

        public FileWatchFunctions()
        {
        }

        [UnmanagedCallersOnly]
        public static unsafe void SourceFileChanged()
        {
            Entry.ScriptEngine.Reload();
            Entry.Application.userSscript = Entry.ScriptEngine.userSscript;
            Entry.Application.Initialize();
        }
    }
}
