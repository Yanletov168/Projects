import os
import shutil
import argparse

def load_user_list(file_path):
    """Загружает список пользователей из текстового файла."""
    if not os.path.exists(file_path):
        print(f"File {file_path} does not exist.")
        return []
    with open(file_path, 'r') as file:
        return [line.strip() for line in file if line.strip()]

def clear_cache_for_users(base_directory, relative_cache_path, users):
    for user_folder in users:
        user_cache_directory = os.path.join(base_directory, user_folder, relative_cache_path)
        if os.path.exists(user_cache_directory):
            print(f"Clearing cache for: {user_cache_directory}")
            for item in os.listdir(user_cache_directory):
                item_path = os.path.join(user_cache_directory, item)
                try:
                    if os.path.isfile(item_path) or os.path.islink(item_path):
                        os.unlink(item_path)  # Удаляем файлы и симлинки
                    elif os.path.isdir(item_path):
                        shutil.rmtree(item_path)  # Удаляем директории
                except Exception as e:
                    print(f"Failed to delete {item_path}. Reason: {e}")
        else:
            print(f"Cache directory does not exist for: {user_folder}")

def copy_user_data_to_profiles(base_directory, relative_user_data_path, target_directory, users):
    for user_folder in users:
        user_data_directory = os.path.join(base_directory, user_folder, relative_user_data_path)
        target_user_directory = os.path.join(target_directory, user_folder)
        if os.path.exists(user_data_directory):
            try:
                if not os.path.exists(target_user_directory):
                    os.makedirs(target_user_directory)  # Создаем директорию, если она не существует
                    print(f"Created directory for user: {target_user_directory}")
                shutil.copytree(user_data_directory, target_user_directory, dirs_exist_ok=True)
                print(f"Copied User Data for {user_folder} to {target_user_directory}")
            except Exception as e:
                print(f"Failed to copy User Data for {user_folder}. Reason: {e}")
        else:
            print(f"User Data directory does not exist for: {user_folder}")

def replace_user_data_with_link(base_directory, relative_user_data_path, target_directory, users):
    for user_folder in users:
        user_data_directory = os.path.join(base_directory, user_folder, relative_user_data_path)
        target_user_directory = os.path.join(target_directory, user_folder)
        if os.path.exists(user_data_directory):
            try:
                # Удаляем оригинальную папку User Data
                shutil.rmtree(user_data_directory)
                print(f"Deleted original User Data directory: {user_data_directory}")
                
                # Создаем жесткую ссылку (Junction) на новую папку
                os.system(f'mklink /J "{user_data_directory}" "{target_user_directory}"')
                print(f"Created junction link from {user_data_directory} to {target_user_directory}")
            except Exception as e:
                print(f"Failed to replace User Data with link for {user_folder}. Reason: {e}")
        else:
            print(f"User Data directory does not exist for: {user_folder}")

def main():
    parser = argparse.ArgumentParser(description="Profile Mover Script")
    parser.add_argument("-f", "--file", required=True, help="Path to the file with the list of users")
    args = parser.parse_args()

    base_directory = r"C:\Users"
    relative_cache_path = r"AppData\Local\Google\Chrome\User Data\Default\Cache"
    relative_user_data_path = r"AppData\Local\Google\Chrome\User Data"
    target_directory = r"E:\ChromeProfiles"

    # Загружаем список пользователей из файла
    users = load_user_list(args.file)
    if not users:
        print("No users to process. Exiting.")
        return

    # Очистка кеша
    clear_cache_for_users(base_directory, relative_cache_path, users)

    # Копирование папок User Data
    copy_user_data_to_profiles(base_directory, relative_user_data_path, target_directory, users)

    # Замена папок User Data на жесткие ссылки
    replace_user_data_with_link(base_directory, relative_user_data_path, target_directory, users)

if __name__ == "__main__":
    main()