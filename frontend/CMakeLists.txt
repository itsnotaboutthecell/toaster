find_package(Qt6 REQUIRED COMPONENTS Widgets Multimedia MultimediaWidgets)

qt_standard_project_setup()

qt_add_executable(toaster-app WIN32
  main.cpp
  MainWindow.cpp
  MainWindow.h
  WaveformView.cpp
  WaveformView.h
)

target_include_directories(toaster-app PRIVATE ${CMAKE_SOURCE_DIR}/libtoaster)

target_link_libraries(toaster-app
  PRIVATE
    Qt6::Widgets
    Qt6::Multimedia
    Qt6::MultimediaWidgets
    toaster
)

if(WIN32)
  target_link_libraries(toaster-app PRIVATE ole32 shell32 uuid)
endif()

if(WIN32)
  find_program(TOASTER_POWERSHELL_EXECUTABLE
    NAMES
      pwsh.exe
      pwsh
      powershell.exe
      powershell
  )

  if(TOASTER_POWERSHELL_EXECUTABLE)
    set(TOASTER_RUN_TOASTER_SCRIPT "${CMAKE_SOURCE_DIR}\\scripts\\run-toaster.ps1")
    set(TOASTER_RUN_TOASTER_LOGGED_SCRIPT "${CMAKE_SOURCE_DIR}\\scripts\\run-toaster-logged.ps1")
    set(TOASTER_RUN_TOASTER_LOG "${CMAKE_BINARY_DIR}\\logs\\toaster-app.log")

    add_custom_target(run-toaster
      COMMAND "${TOASTER_POWERSHELL_EXECUTABLE}"
              -NoProfile
              -ExecutionPolicy Bypass
              -File "${TOASTER_RUN_TOASTER_SCRIPT}"
              -AppPath "$<TARGET_FILE:toaster-app>"
              -WorkingDirectory "$<TARGET_FILE_DIR:toaster-app>"
      DEPENDS toaster-app
      USES_TERMINAL
      VERBATIM
      COMMENT "Launching toaster-app"
    )

    add_custom_target(run-toaster-logged
      COMMAND "${TOASTER_POWERSHELL_EXECUTABLE}"
              -NoProfile
              -ExecutionPolicy Bypass
              -File "${TOASTER_RUN_TOASTER_LOGGED_SCRIPT}"
              -AppPath "$<TARGET_FILE:toaster-app>"
              -WorkingDirectory "$<TARGET_FILE_DIR:toaster-app>"
              -LogPath "${TOASTER_RUN_TOASTER_LOG}"
      DEPENDS toaster-app
      USES_TERMINAL
      VERBATIM
      COMMENT "Launching toaster-app with logs"
    )
  else()
    message(WARNING "PowerShell not found; run-toaster targets are unavailable.")
  endif()
endif()

if(WIN32)
  find_program(TOASTER_WINDEPLOYQT_EXECUTABLE
    NAMES
      windeployqt6
      windeployqt
    HINTS
      "C:/msys64/mingw64/bin"
  )

  if(TOASTER_WINDEPLOYQT_EXECUTABLE)
    add_custom_command(TARGET toaster-app POST_BUILD
      COMMAND "${TOASTER_WINDEPLOYQT_EXECUTABLE}"
              --verbose 0
              --force
              --no-translations
              --compiler-runtime
              "$<TARGET_FILE:toaster-app>"
      COMMENT "Deploying Qt and compiler runtime for toaster-app"
    )
  else()
    message(WARNING "windeployqt not found; toaster-app may not run outside the build shell.")
  endif()

  set(TOASTER_RUNTIME_DEPLOY_SCRIPT
    ${CMAKE_CURRENT_BINARY_DIR}/deploy-runtime-deps.cmake
  )

  file(GENERATE OUTPUT "${TOASTER_RUNTIME_DEPLOY_SCRIPT}" CONTENT [=[
if(POLICY CMP0207)
  cmake_policy(SET CMP0207 NEW)
endif()

file(GLOB_RECURSE toaster_runtime_modules
  "$<TARGET_FILE_DIR:toaster-app>/*.dll"
)

file(GET_RUNTIME_DEPENDENCIES
  RESOLVED_DEPENDENCIES_VAR toaster_resolved_deps
  UNRESOLVED_DEPENDENCIES_VAR toaster_unresolved_deps
  CONFLICTING_DEPENDENCIES_PREFIX toaster_conflicts
  EXECUTABLES "$<TARGET_FILE:toaster-app>"
  LIBRARIES ${toaster_runtime_modules}
  DIRECTORIES "C:/msys64/mingw64/bin"
  POST_EXCLUDE_REGEXES
    "api-ms-.*"
    "ext-ms-.*"
    ".*[/\\\\]Windows[/\\\\]System32[/\\\\].*"
)

foreach(toaster_dep IN LISTS toaster_resolved_deps)
  execute_process(
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
            "${toaster_dep}"
            "$<TARGET_FILE_DIR:toaster-app>"
  )
endforeach()

if(toaster_unresolved_deps)
  list(REMOVE_DUPLICATES toaster_unresolved_deps)
  message(STATUS "Unresolved runtime deps: ${toaster_unresolved_deps}")
endif()

if(toaster_conflicts_FILENAMES)
  list(REMOVE_DUPLICATES toaster_conflicts_FILENAMES)
  message(STATUS "Conflicting runtime deps ignored: ${toaster_conflicts_FILENAMES}")
endif()
]=])

  add_custom_command(TARGET toaster-app POST_BUILD
    COMMAND "${CMAKE_COMMAND}" -P "${TOASTER_RUNTIME_DEPLOY_SCRIPT}"
    COMMENT "Copying transitive runtime DLLs for toaster-app"
  )

  find_program(TOASTER_FFMPEG_EXECUTABLE
    NAMES
      ffmpeg.exe
      ffmpeg
    HINTS
      "C:/msys64/mingw64/bin"
  )
  find_program(TOASTER_FFPROBE_EXECUTABLE
    NAMES
      ffprobe.exe
      ffprobe
    HINTS
      "C:/msys64/mingw64/bin"
  )

  if(TOASTER_FFMPEG_EXECUTABLE)
    add_custom_command(TARGET toaster-app POST_BUILD
      COMMAND "${CMAKE_COMMAND}" -E copy_if_different
              "${TOASTER_FFMPEG_EXECUTABLE}"
              "$<TARGET_FILE_DIR:toaster-app>"
      COMMENT "Copying ffmpeg for toaster-app"
    )
  endif()

  if(TOASTER_FFPROBE_EXECUTABLE)
    add_custom_command(TARGET toaster-app POST_BUILD
      COMMAND "${CMAKE_COMMAND}" -E copy_if_different
              "${TOASTER_FFPROBE_EXECUTABLE}"
              "$<TARGET_FILE_DIR:toaster-app>"
      COMMENT "Copying ffprobe for toaster-app"
    )
  endif()

  find_program(TOASTER_WHISPER_CLI_EXECUTABLE
    NAMES
      whisper-cli.exe
      whisper-cli
    HINTS
      "C:/msys64/mingw64/bin"
  )

  if(TOASTER_WHISPER_CLI_EXECUTABLE)
    add_custom_command(TARGET toaster-app POST_BUILD
      COMMAND "${CMAKE_COMMAND}" -E copy_if_different
              "${TOASTER_WHISPER_CLI_EXECUTABLE}"
              "$<TARGET_FILE_DIR:toaster-app>"
      COMMAND "${CMAKE_COMMAND}" -E copy_if_different
              "C:/msys64/mingw64/bin/libwhisper-1.dll"
              "$<TARGET_FILE_DIR:toaster-app>"
      COMMENT "Copying whisper-cli for toaster-app"
    )

    file(GLOB TOASTER_GGML_DLLS "C:/msys64/mingw64/bin/ggml*.dll")
    if(TOASTER_GGML_DLLS)
      add_custom_command(TARGET toaster-app POST_BUILD
        COMMAND "${CMAKE_COMMAND}" -E copy_if_different
                ${TOASTER_GGML_DLLS}
                "$<TARGET_FILE_DIR:toaster-app>"
        COMMENT "Copying ggml runtime DLLs for toaster-app"
      )
    endif()
  endif()
endif()
