task("setup")
do
    local ffmpeg_dir = ffmpeg_dir
    local engine_root_dir = engine_root_dir
    local dotnet_sdk_dir = dotnet_sdk_dir
    on_run(function()
        import("net.http")
        import("utils.archive")
        import("lib.detect.find_program")
        import("core.project.task")

        local function setup_ffmpeg(build_type, target)
            local target_dir = target .. "/target/"
            os.mkdir(target_dir .. build_type)
            os.cp(ffmpeg_dir .. "/bin/*.dll", target_dir .. build_type)
        end

        local function setup_dotnet(build_type, target)
            local target_dir = target .. "/target/"
            os.mkdir(target_dir .. build_type)
            os.cp(dotnet_sdk_dir .. "/packs/Microsoft.NETCore.App.Host.win-x64/8.0.6/runtimes/win-x64/native/nethost.dll", target_dir .. build_type)
        end

        setup_ffmpeg("debug", "rs_editor")
        setup_ffmpeg("release", "rs_editor")
        setup_ffmpeg("debug", "rs_desktop_standalone")
        setup_ffmpeg("release", "rs_desktop_standalone")
        setup_ffmpeg("debug", "rs_media_cmd")
        setup_ffmpeg("release", "rs_media_cmd")
        setup_dotnet("debug", "rs_editor")
        setup_dotnet("release", "rs_editor")
        setup_dotnet("debug", "rs_desktop_standalone")
        setup_dotnet("release", "rs_desktop_standalone")
        os.cp(path.join(ffmpeg_dir, "lib/*.so"), path.join(engine_root_dir, "Android/Template/rs_android/src/main/jniLibs/arm64-v8a"))
    end)
    set_menu {
        usage = "xmake setup",
        description = "Initialize Project",
        options = {
            { nil, "setup", nil, nil, nil },
        }
    }
end