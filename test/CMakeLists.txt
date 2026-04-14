add_executable(test-edit test-edit.c)
target_include_directories(test-edit PRIVATE ${CMAKE_SOURCE_DIR}/libtoaster)
target_link_libraries(test-edit PRIVATE toaster)
add_test(NAME test-edit COMMAND $<TARGET_FILE:test-edit>)

add_executable(test-signals test-signals.c)
target_include_directories(test-signals PRIVATE ${CMAKE_SOURCE_DIR}/libtoaster)
target_link_libraries(test-signals PRIVATE toaster)
add_test(NAME test-signals COMMAND $<TARGET_FILE:test-signals>)

add_executable(test-timeline test-timeline.c)
target_include_directories(test-timeline PRIVATE ${CMAKE_SOURCE_DIR}/libtoaster)
target_link_libraries(test-timeline PRIVATE toaster)
add_test(NAME test-timeline COMMAND $<TARGET_FILE:test-timeline>)

add_executable(test-project test-project.c)
target_include_directories(test-project PRIVATE ${CMAKE_SOURCE_DIR}/libtoaster)
target_link_libraries(test-project PRIVATE toaster)
add_test(NAME test-project COMMAND $<TARGET_FILE:test-project>)

add_executable(test-analysis test-analysis.c)
target_include_directories(test-analysis PRIVATE ${CMAKE_SOURCE_DIR}/libtoaster)
target_link_libraries(test-analysis PRIVATE toaster)
add_test(NAME test-analysis COMMAND $<TARGET_FILE:test-analysis>)

add_executable(test-export test-export.c)
target_include_directories(test-export PRIVATE ${CMAKE_SOURCE_DIR}/libtoaster)
target_link_libraries(test-export PRIVATE toaster)
add_test(NAME test-export COMMAND $<TARGET_FILE:test-export>)

add_executable(test-model test-model.c)
target_include_directories(test-model PRIVATE ${CMAKE_SOURCE_DIR}/libtoaster)
target_link_libraries(test-model PRIVATE toaster)
add_test(NAME test-model COMMAND $<TARGET_FILE:test-model>)

if(WIN32 AND TARGET toaster-app)
  find_program(TOASTER_POWERSHELL_EXECUTABLE
    NAMES
      pwsh.exe
      pwsh
      powershell.exe
      powershell
  )

  if(TOASTER_POWERSHELL_EXECUTABLE)
    set(TOASTER_FRONTEND_SMOKE_SCRIPT "${CMAKE_SOURCE_DIR}\\scripts\\run-toaster-logged.ps1")
    set(TOASTER_FRONTEND_SMOKE_LOG "${CMAKE_BINARY_DIR}\\logs\\frontend-automation-smoke.log")

    add_test(
      NAME test-frontend-automation
      COMMAND "${TOASTER_POWERSHELL_EXECUTABLE}"
              -NoProfile
              -ExecutionPolicy Bypass
              -File "${TOASTER_FRONTEND_SMOKE_SCRIPT}"
              -AppPath "$<TARGET_FILE:toaster-app>"
              -WorkingDirectory "$<TARGET_FILE_DIR:toaster-app>"
              -RequireAutomationMedia
              -LogPath "${TOASTER_FRONTEND_SMOKE_LOG}"
    )

    set_tests_properties(test-frontend-automation PROPERTIES
      LABELS "frontend;automation"
      SKIP_RETURN_CODE 125
      TIMEOUT 600
    )
  else()
    message(STATUS "PowerShell not found; skipping test-frontend-automation registration.")
  endif()
endif()
